# API — 自分用の覚書

英語の [README.md](../../../README.md) は `product-merger` の仕様書なので、ここには
**運用していて迷うところ**だけを書いておく。いまの主題は「プロダクトのリポジトリから
通知が来たときに何が起きるか」。

## 通知の受け口

各プロダクトのリポジトリからの通知は `repository_dispatch` ではなく
**`workflow_dispatch`** で受けている（[deploy.yml](../../workflows/deploy.yml)）。入力は2つだけ。

| 入力 | 意味 |
|---|---|
| `repo` | 通知元の `owner/name` |
| `update_brew` | Homebrew formula の更新が必要か |

通知する側には GitHub App の `CLIENT_ID` と `APP_PRIVATE_KEY` を secret として入れておく
必要がある。登録は [scripts/install-pipeline-bot-to-repository/run.sh](../../../scripts/install-pipeline-bot-to-repository/run.sh) で行う
（スクリプト内のコメントに、連携したリポジトリの一覧が残っている）。

通知を受けると3つのジョブが動く。

1. **`update-homebrew-tap`** — `update_brew == 'true'` のときだけ。`tamada/homebrew-tap` の
   `update-formula.yaml` を dispatch して、**完了まで待つ**。homebrew-tap 自身もこの API の
   プロダクトなので、formula のコミットが入る前にビルドすると自分の古い情報を公開して
   しまう。
2. **`build-and-deploy`** — pkl → JSON → `product-merger` → fauxrest → GitHub Pages。
   `always() && !cancelled()` なので、前段がスキップ・失敗しても deploy は走る。
3. **`notify-to-homepage`** — `tamada/tamada.github.io` の `publish_site.yaml` を dispatch。

## 未登録のリポジトリから通知が来たら

**warning が出るだけで、API には出てこない。** ビルドは成功扱いで終わる。

`repo` の値がビルド内容に影響することはない。`build-and-deploy` は毎回
`assets/base-products.pkl` の全エントリを対象にする。しかも CI では
`--base-project-json` を付けて渡すので全エントリが base product として扱われ、
`product-merger` の差分更新（`last_updated` と `updatedAt` の比較）は効かず、
**毎回全件を GitHub から取り直している**。`data/*.json` は `.gitignore` の
`/data/*.json` で無視されていて前回の `products.json` が残らないので、そもそも
差分の基準がない（`data/` で commit してあるのは手書きの `_config.json` と
`assets/` のロゴだけ）。`repo` を見ているのは Homebrew tap のジョブと、この
warning を出すステップだけ。

warning は `Warn if the notified repository is not registered` ステップが出す。
`data/base-products.json` に `owner`/`name` が一致するエントリがあるかを `jq` で見て、
無ければ

```
<owner>/<name> is not registered in assets/base-products.pkl, hence it does not show up in this API.
```

を `::warning::` で出す。GitHub Actions の run 画面の Annotations とサマリに出るので、
**通知が来たこと自体は見逃さない**。`owner/name` の形になっていない値が来た場合も
別の warning を出す。

### なぜ自動登録せず warning だけにしてあるのか

自動登録にすると、プロダクトが知らないうちに増えていき、**バグで増えたのか期待通りに
増えたのかが区別できなくなる**。

- `assets/base-products.pkl` のコミット履歴が、プロダクト集合の唯一の監査記録になっている。
  自動登録するとこの情報が workflow run のログに移り、実質追えなくなる。
- `product-merger` の `update_all` は入力エントリを `map` するだけで**削除経路がない**。
  リネームや削除されたリポジトリも「警告して前回の情報を保持」する仕様なので、
  一度誤って入ったエントリは手で消すまで残り、しかも生きているように見え続ける。
  誤検知のコストが非対称に高い。
- 通知は opt-in（bot と secret を入れたリポジトリだけが通知してくる）だが、その操作は
  相手のリポジトリ側なので、この API のリポジトリの履歴からは見えない。

要するに、手で `base-products.pkl` にコミットする作業は**手間ではなくゲート**。
もし将来自動化するとしても、直接コミットではなく PR にして review を挟む形にする。

## warning が出たときの登録手順

1. ロゴ画像を `data/assets/` に置く。ここに置いたファイルは fauxrest が `docs/assets/` に
   出すので、`https://tamada.github.io/api/assets/<ファイル名>` で配信される。
2. `assets/base-products.pkl` の `products` に `ProductBase` を追記する。

   ```pkl
   new {
       owner = "tamada"
       name = "<name>"
       links = new {
           "www" "repository" "sbom"
           new Link {
               label = "logo"
               url = "https://tamada.github.io/api/assets/<name>.svg"
           }
       }
   }
   ```

3. リンクは2通りの書き方がある。**`"www"` `"repository"` `"sbom"` の3つは文字列だけ
   書けばよく、`product-merger` が `url` を組み立てて `Link` に展開する。**

   | 文字列 | 展開される url |
   |---|---|
   | `"www"` | リポジトリの `homepageUrl`、空なら `https://<owner>.github.io/<name>` |
   | `"repository"` | リポジトリの `url`、GitHub に問い合わせていなければ `https://github.com/<owner>/<name>` |
   | `"sbom"` | `https://api.github.com/repos/<owner>/<name>/dependency-graph/sbom`（GitHub は返さないので常に組み立てる） |

   残りの `logo` `docs` `registry` `container` `artifact` には展開する元が無い。文字列
   だけで書くと「URL を組み立てられない」と警告を出して捨てられるので、
   `new Link { label = ...; url = ... }` の形で url まで書く。`registry` は crates.io や
   npm、`container` は quay.io など、自分しか知らない情報なので手で書くしかない。

   `www` `repository` `sbom` をオブジェクトで書くこともできる。その場合は展開せず、
   書いた url がそのまま使われる（`homepageUrl` に上書きされない）。
4. ORAS で引くもの（quay に置いたデータベースなど）は `artifact` に `ref` を書く。

   ```pkl
   new Link {
       label = "artifact"
       ref = "quay.io/tama5/pelicode-db:v1"
   }
   ```

   `ref` は `oras pull` / `docker pull` に渡す文字列、`url` はブラウザで開くページ。
   どちらも省略可で、ただし両方省略すると pkl の型チェックで落ちる。

   **ブラウズできるページがあるなら `url` も書く。** quay なら
   `https://quay.io/repository/<owner>/<name>` がある。そうしておけば、消費側
   （tamada.github.io）は「url があればリンクにする」だけで済み、url の無いリンクを
   特別扱いしなくてよい。pelicode はそうしてある。`url` を省略するのは、置き場所に
   web ページが無い場合だけ。

   `url` も `ref` も、書いたときだけ出力に出る。だから使っていないプロダクトの出力は
   変わらない。
5. GitHub から取ってきた値を上書きしたいときは `overrides` を使う。キー単位で
   **丸ごと差し替える**（中身をマージするのではない）。

   ```pkl
   new {
       owner = "tamada"
       name = "pelicode"
       overrides = new Mapping {
           ["description"] = "GitHub の説明文の代わりにこれを出す"
           ["languages"] = new Listing { "Rust" }
       }
       links = new { "repository" "sbom" }
   }
   ```

   `links` を丸ごと置き換えることもできる。この場合 `links` の方に書いた内容は
   出力に出ない。

   ```pkl
   overrides = new Mapping {
       ["links"] = new Listing {
           new Link {
               label = "registry"
               url = "https://crates.io/crates/pelicode"
           }
       }
   }
   ```

   引っかかりやすい点:

   - `owner` と `name` はリポジトリの同定に使うので上書きできない（警告を出して無視
     される）。`overrides` 自身も同じ。
   - **`logo` `url` `repository` `sbom` を上書きしても効かない。** 昔は出力のフィールド
     だったが今は `links` に移っていて、出力から捨てられる（`src/product.rs` の
     `RETIRED_KEYS`）。これらを変えたいときは上の例のように `links` を置き換える。
   - `overrides` は出力にも残るので、毎回の更新で再適用される。
6. コミットして `main` に push すれば、`push` トリガで再ビルド・再デプロイされる。

## ローカルで確認する

[Justfile](../../../Justfile) にまとまっている。

```sh
just api
```

`pkl2json` → `build` → `base-product-to-product-json` → fauxrest の順に走って `docs/` が
できる。`GH_TOKEN` が必要（`gh auth login` 済みなら不要）。`docs/` は生成物なので
コミットしない。
