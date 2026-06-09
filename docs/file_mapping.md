# 内部文書：静的REST API 物理ファイル・マッピング

このドキュメントは、定義された REST API エンドポイントが、GitHub Pages 等の静的ホスティング上でどのような物理ファイルとして保存されるかを定義します。

## マッピング・ルール

1.  **基本原則**: 各エンドポイントは可能な限り `{resource}` という**拡張子なし**のファイル名で保存します（例: `profile`）。
2.  **階層構造の維持**: あるエンドポイント `P` の配下にサブ・リソース（`P/ids/{id}` など）が存在する場合、`P` 自体はファイルとして存在できないため、`P/index.json` として保存します。
3.  **パス・パラメータ**: `{id}` や `{year}` などのパラメータは、実際の値に基づいた拡張子なしのファイル名（例: `2024`）として展開されます。

## マッピング一覧

### 0. Discovery (エンドポイント探索)

| エンドポイント | 物理ファイル・パス | 理由 |
| :--- | :--- | :--- |
| `/api` | `api/index.json` | ルート。配下に多数のリソースが存在するため |
| `/api/ja` | `api/ja` | 終端リソース（拡張子なし） |

### 1. Personal (個人データ)

| エンドポイント | 物理ファイル・パス | 理由 |
| :--- | :--- | :--- |
| `/api/profile` | `api/profile` | 終端リソース（拡張子なし） |
| `/api/activities` | `api/activities` | 終端リソース（拡張子なし） |
| `/api/degrees` | `api/degrees` | 終端リソース（拡張子なし） |
| `/api/job-histories` | `api/job-histories/index.json` | 下位に `{as}`, `current` が存在するため |
| `/api/job-histories/{as}` | `api/job-histories/{as}` | 終端リソース（拡張子なし） |
| `/api/job-histories/current` | `api/job-histories/current` | 終端リソース（拡張子なし） |
| `/api/skills` | `api/skills` | 終端リソース（拡張子なし） |

### 2. Laboratory (研究室データ)

| エンドポイント | 物理ファイル・パス | 理由 |
| :--- | :--- | :--- |
| `/api/members/all` | `api/members/all` | 終端リソース（拡張子なし） |
| `/api/members/{category}` | `api/members/{category}` | 終端リソース（拡張子なし） |
| `/api/members/ids/{id}` | `api/members/ids/{id}` | 終端リソース（拡張子なし） |
| `/api/grants` | `api/grants/index.json` | 下位に `actives` が存在するため |
| `/api/grants/actives` | `api/grants/actives` | 終端リソース（拡張子なし） |
| `/api/papers` | `api/papers/index.json` | 下位に `recent`, `years/`, `types/`, `languages/`, `ids/` が存在するため |
| `/api/papers/recent` | `api/papers/recent` | 終端リソース（拡張子なし） |
| `/api/theses` | `api/theses/index.json` | 下位に `years` が存在するため |
| `/api/theses/years/{year}` | `api/theses/years/{year}` | 終端リソース（拡張子なし） |
| `/api/papers/types/{type}` | `api/papers/types/{type}` | 終端リソース（拡張子なし） |
| `/api/papers/years/{year}` | `api/papers/years/{year}` | 終端リソース（拡張子なし） |
| `/api/papers/languages/japanese` | `api/papers/languages/japanese` | 終端リソース（拡張子なし） |
| `/api/papers/languages/english` | `api/papers/languages/english` | 終端リソース（拡張子なし） |
| `/api/papers/ids/{id}` | `api/papers/ids/{id}` | 終端リソース（拡張子なし） |
| `/api/activities` | `api/activities` | 終端リソース（拡張子なし） |

## 補足事項
- クライアントは `/api/papers` でリクエストを送り、サーバー（GitHub Pages）はディレクトリ・インデックス機能によって `api/papers/index.json` を返します。
- これにより、静的配信でありながら拡張子を意識させない RESTful な URI 体系が維持されます。
