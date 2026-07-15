# App ID を設定（メモした数字）
APP_ID="4300601"

if [[ "$@" == 1 ]]; then
    echo "run.sh <PEM>"
    exit 1
fi

# ダウンロードした .pem ファイルのパスを指定
PEM_FILE=$1

# 連携させたいリポジトリのリスト
REPOS=(
  "tamada/fauxrest"
  "tamada/spellout"
  "tamada/pick-a-boo"
  "tamada/oinkie"
  "tamada/lis"
  "tamada/gixor"
  "tamada/totebag"
  "tamada/btmeister"
  "tamada/heatman"
  "tamada/9rules"
  "tamada/pochi"
  "tamada/sibling"
  "tamada/wildcat"
)

for repo in "${REPOS[@]}"; do
  gh secret set APP_ID --body "$APP_ID" --repo "$repo"
  gh secret set APP_PRIVATE_KEY --body "$(cat $PEM_FILE)" --repo "$repo"
  echo "$repo にシークレットを登録完了！"
done
