# API — 自分用の覚書

英語の [README.md](../../../README.md) は `product-merger` の仕様書なので、ここには
**運用していて迷うところ**だけを書いておく。いまの主題は「プロダクトのリポジトリから
通知が来たときに何が起きるか」と「毎回のビルドで何を GitHub から取り直すのか」。

## 通知の受け口

各プロダクトのリポジトリからの通知は `repository_dispatch` ではなく
**`workflow_dispatch`** で受けている（[deploy.yml](../../workflows/deploy.yml)）。入力は3つだけ。

| 入力 | 意味 |
|---|---|
| `repo` | 通知元の `owner/name` |
| `update_brew` | Homebrew formula の更新が必要か |
| `full_refresh` | キャッシュを無視して全件を GitHub から取り直すか（既定 false） |

通知する側には GitHub App の `CLIENT_ID` と `APP_PRIVATE_KEY` を secret として入れておく
必要がある。登録は [scripts/install-pipeline-bot-to-repository/run.sh](../../../scripts/install-pipeline-bot-to-repository/run.sh) で行う
（スクリプト内のコメントに、連携したリポジトリの一覧が残っている）。

通知を受けると3つのジョブが動く。

1. **`update-homebrew-tap`** — `update_brew == 'true'` のときだけ。`tamada/homebrew-tap` の
   `update-formula.yaml` を dispatch して、**完了まで待つ**。homebrew-tap 自身もこの API の
   プロダクトなので、formula のコミットが入る前にビルドすると自分の古い情報を公開して
   しまう。
2. **`build-and-deploy`** — pkl → JSON → 公開中の API をキャッシュとして取得 →
   `product-merger` → fauxrest → GitHub Pages。
   `always() && !cancelled()` なので、前段がスキップ・失敗しても deploy は走る。
3. **`notify-to-homepage`** — `tamada/tamada.github.io` の `publish_site.yaml` を dispatch。

## 未登録のリポジトリから通知が来たら

**warning が出るだけで、API には出てこない。** ビルドは成功扱いで終わる。

`repo` の値がビルド内容に影響することはない。`build-and-deploy` は毎回
`assets/base-products.pkl` の全エントリを対象にする。`repo` を見ているのは
Homebrew tap のジョブと、この warning を出すステップだけ。

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

## 差分更新のしくみ（公開中の API をキャッシュにしている）

`data/*.json` は `.gitignore` の `/data/*.json` で無視されていて、前回の
`products.json` はリポジトリに残らない（`data/` で commit してあるのは手書きの
`_config.json` と `assets/` のロゴだけ）。そこで**前回の出力は公開中の API から
読み戻している**。

```
curl -fsS https://tamada.github.io/api/products/index.json -o $RUNNER_TEMP/products-cache.json
cargo run -- --pretty --base-project-json --cache $RUNNER_TEMP/products-cache.json data/base-products.json
```

`https://tamada.github.io/api/products/index.json` は `data/products.json` が素通しで
出ているものなので、そのまま `--cache` に渡せる。`$RUNNER_TEMP` に置くのは、`data/` が
fauxrest の入力ツリーなので、そこにキャッシュを置くと API に出てしまうから。

**キャッシュが決めるのは「GitHub に聞きに行くかどうか」だけ。** 設定の正は
`assets/base-products.pkl` のままで、

- プロダクト集合は pkl だけが決める（pkl から消したものはキャッシュに残っていても
  出力から消える）。新規プロダクトの手動登録というゲートは壊れていない。
- `links` / `logo` / `overrides` は**毎回 pkl から再適用される**。だから repo が
  無変更でも、pkl でリンクや overrides を直せば次のビルドで反映される。
- キャッシュから流用するのは GitHub 由来の値（`description` `releases` `license`
  `languages` `topics`）だけ。
- 取得に失敗・404・空（初回）のときは warning を出してフル取得にフォールバックする。
  ビルドは止まらない。

### 全件取り直したいとき

`workflow_dispatch` の `full_refresh` を true にする（`--no-cache` が渡る）。
ローカルなら `just base-product-to-product-json --no-cache`。

`releases` は**取り直せば必ず作り直せる値**になっている。GraphQL で最新10件を
まとめて取り、draft を除いて毎回組み直す（以前は `latestRelease` 1件を前回出力に
追記して履歴を育てていたので、ビルドを取りこぼすと履歴が永久に欠けた）。
キャッシュを使っている間は repo が無変更なら以前の1件のままなので、**履歴をまとめて
埋めたいなら一度 `full_refresh` を走らせる**。

### 比較しているタイムスタンプ

**`updatedAt` と `pushedAt` の新しい方**を両辺で使っている。どちらかが上位ではない。

- `updatedAt` は description や topics の変更も拾う
- `pushedAt` は default 以外のブランチへの push を拾う
- `updatedAt` が `pushedAt` より**古い** repo が実在する（`9rules` は
  `updatedAt` が1年近く前）

片方だけを見ると、保存した側が進んでいる repo が永久に取り直されなくなる。
なので取得側・detail 側ともに `max(updatedAt, pushedAt)` にしてある。
出力の `last_updated` もこの値。フェッチできたがタイムスタンプが取れなかった場合は
`last_updated` を空のままにして、次回取り直させる（以前は `now()` を入れていたので、
中途半端な値がそのまま固着した）。

### なぜ owner 全件 listing をやめたのか

タイムスタンプは **pkl に書いてある repo だけを名前指定で聞く**（alias で18件を
1リクエスト、`github::fetch_last_modified`）。以前は `user(login:)` で owner の
リポジトリ全件を listing していたが、これは捨てた。

理由は `user(login:).repositories` の**デフォルトが「tamada が owner のもの」ではなく
「tamada が関わっている全リポジトリ」**だったこと。実測:

| 条件 | 件数 |
|---|---|
| デフォルト（旧 `product_list.graphql`） | 277 |
| `ownerAffiliations: [OWNER]` を付ける | 215 |

差の62件は他人や org が owner のもので、`tamadalab` `ksuap` `sagisaka`、そして
**`TokunagaResearch`** が含まれていた。だから CI で

```
The 'TokunagaResearch' organization forbids access via a fine-grained personal
access tokens if the token's lifetime is greater than 366 days.
```

と言われて**query 自体が丸ごと拒否されていた**（プロダクトは全部 public なのに）。
拒否されると全件のタイムスタンプが取れず、差分更新は何も効かない。

`ownerAffiliations` で絞っても215件を3ページ取って使うのは18件なので、名前指定が
素直に上位互換。1ページで済み、他人や org のリポジトリに一切触らない。

### Build API のトークン

名前指定にしたことで public な repo しか読まなくなったので、`secrets.ACCESS_TOKEN`
（長命の fine-grained PAT）をやめて **`secrets.GITHUB_TOKEN`**（run に自動で付く
トークン）にした。`permissions` には既に `contents: write` がある。

### query に権限の要るフィールドを混ぜない

`GITHUB_TOKEN` は他リポジトリの public データを読める。**ただし query に権限の要る
フィールドが1つでも混ざっていると、リクエストが丸ごと落ちる。**

最初にこのトークンで回した run（39188613796）では、タイムスタンプは18件すべて
取れたのに、変更のあった4件の detail query が全滅した。

```
ERROR api::updater] tamada/fauxrest: github error: gh: Resource not accessible by integration
```

原因は `queries/products.graphql` が聞いていた **`collaborators { totalCount }`**。
これは**リポジトリへの push 権が必要**なフィールドで、public repo でも他人のものは
読めない。しかも `RepoDetail` が deserialize すらしていない、**使っていないフィールド**
だった。手元でも再現する:

```sh
# push 権の無い public repo に collaborators を聞くと FORBIDDEN
gh api graphql -f query='query{repository(owner:"rust-lang",name:"rust"){collaborators{totalCount}}}'
```

そこで `products.graphql` を **`RepoDetail` が実際に読むフィールドだけ**に削った
（`collaborators` のほか `isPrivate` `visibility` `diskUsage` `primaryLanguage`
`stargazerCount` `forkCount` `watchers`、`languages`/`topics`/`releases` の
`color` と `url` も未使用だったので一緒に削除）。削った後の query は push 権の無い
repo でも通る。

**今後 query にフィールドを足すときは、`RepoDetail` が読むものだけにすること。**
権限の要るものを混ぜると detail query が落ち、全プロダクトがキャッシュの値のまま
更新されなくなる（出力は壊れないが、気づきにくい）。

なお失敗しても出力が壊れないのは設計どおりで、このときも公開 API は18件のまま、
4件はキャッシュの値（`description` も `links` も `releases` も）を保持していた。

## ローカルで確認する

[Justfile](../../../Justfile) にまとまっている。

```sh
just api
```

`pkl2json` → `build` → `base-product-to-product-json` → fauxrest の順に走って `docs/` が
できる。`GH_TOKEN` が必要（`gh auth login` 済みなら不要）。`docs/` は生成物なので
コミットしない。

`base-product-to-product-json` は CI と同じように公開中の API を `--cache` に渡すので、
ローカルでも変更のあった repo しか取りに行かない。全件取り直すなら

```sh
just base-product-to-product-json --no-cache
```
