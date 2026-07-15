if [[ "$@" != 4 ]]; then
    echo "run.sh <REPO> <PEM> <CLIENT_ID>"
    exit 1
elif [[ ! -f "$2" ]]; then
    echo "指定されたファイルが存在しません: $2"
    exit 1
elif [[ ! "$2" =~ \.pem$ ]]; then
    echo "指定されたファイルは .pem ファイルではありません: $2"
    exit 1
fi

# ダウンロードした .pem ファイルのパスを指定
PEM_FILE=$2
CLIENT_ID=$3
REPO=$1

# 連携させたいリポジトリのリスト
# REPOS=(
  # "tamada/fauxrest"
  # "tamada/spellout"
  # "tamada/pick-a-boo"
  # "tamada/oinkie"
  # "tamada/lis"
  # "tamada/gixor"
  # "tamada/totebag"
  # "tamada/btmeister"
  # "tamada/heatman"
  # "tamada/9rules"
  # "tamada/pochi"
  # "tamada/sibling"
  # "tamada/wildcat"
  # "tamada/api"
# )

gh secret set CLIENT_ID --body "$CLIENT_ID" --repo "$repo"
gh secret set APP_PRIVATE_KEY --body "$(cat $PEM_FILE)" --repo "$repo"
echo "$repo にシークレットを登録完了！"
