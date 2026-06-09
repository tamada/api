# 擬似REST API 仕様書

このドキュメントでは、玉田春昭および玉田研究室に関するデータにアクセスするための擬似REST APIエンドポイントを定義します。

## 概要

このAPIは、主にGitHub Pagesでのホスティングを想定した**静的REST API**として設計されています。データは事前に生成されたJSONファイルとして提供されます。静的ファイルによる運用の利益を最大化するため、サーバーサイドでの動的なフィルタリングは行わず、クライアントサイドでのフィルタリング、または物理的なディレクトリ構造によるアクセスを推奨します。

## エンドポイント探索 (Discovery)
- `GET /api`: 利用可能なすべてのエンドポイントと説明のリスト（英語）。
- `GET /api/ja`: 利用可能なすべてのエンドポイントと説明のリスト（日本語）。

## 1. 個人データ (Personal)

玉田春昭個人のデータです。

### エンドポイント
- `GET /api/profile`: 個人のプロフィール詳細。
- `GET /api/activities`: 学会活動などの活動実績。
- `GET /api/degrees`: 取得学位のリスト。
- `GET /api/job-histories`: 職歴のリスト。
- `GET /api/skills`: 技術スキルおよび言語スキル。

---

## 2. 研究室データ (Laboratory)

玉田研究室のメンバー、研究成果、および活動に関するデータです。

### エンティティ
- **Member (メンバー)**: 現役生、卒業生、および共同研究者。
- **Paper (研究論文)**: 学術雑誌、国際会議、国内会議での発表論文。
- **Thesis (学位論文)**: 卒業論文、修士論文、博士論文。
- **Grant (助成金)**: 科研費などの研究助成金。

### エンドポイント

#### メンバー (Members)
- `GET /api/members/all`: すべてのメンバーのリスト。
- `GET /api/members/{category}`: カテゴリ（`actives`, `almus` など）によるフィルタリング。
- `GET /api/members/ids/{id}`: 特定のメンバーの詳細情報。

#### 研究成果 (Publications)
- `GET /api/papers`: すべての研究論文（学位論文を除く）。
- `GET /api/papers/recent`: 最新の50件程度の論文（高速アクセスのため）。
- `GET /api/theses`: すべての学位論文。
- `GET /api/theses/years/{year}`: 特定の年（西暦）の学位論文。
- `GET /api/papers/years/{year}`: 特定の年（西暦）の出版物。
- `GET /api/papers/types/{type}`: 特定の種類（journal, conferenceなど）の出版物。
- `GET /api/papers/languages/japanese`: 日本語の出版物。
- `GET /api/papers/languages/english`: 英語の出版物。

#### 助成金・活動 (Grants & Activities)
- `GET /api/grants`: すべての研究助成金。
- `GET /api/grants/actives`: 現在活動中の研究助成金のリスト。
- `GET /api/activities`: 研究室全体の活動やイベントのリスト。

---

## データ形式

すべてのレスポンスは **JSON形式** です。
日付は `YYYY-MM-DD` または `YYYY-MM` 形式に従います。
内部の Pkl 構造は、ビルドプロセス中に JSON オブジェクト/配列に変換されます。

## API 利用に関する注意
この API は RESTful な原則に従って設計されています。クライアントは、前述の定義通りのパス（例: `/api/tamadalab/papers`）でエンドポイントにアクセスしてください。内部的な実装（静的ファイル配信など）を意識させることのない、クリーンな抽象化を提供します。データ件数が多いリソースについては、一度取得した後にクライアント側でフィルタリングやソートを行う手法を推奨します。
