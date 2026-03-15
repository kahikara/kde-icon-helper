cd /home/ming/github && bash <<'EOF'
set -euo pipefail

cd kde-icon-helper

echo "==> git repo pruefen"
if [ ! -d .git ]; then
  git init
fi

echo "==> branch auf main"
git branch -M main

echo "==> remote origin setzen"
REMOTE_URL="https://github.com/kahikara/kde-icon-helper.git"

if git remote get-url origin >/dev/null 2>&1; then
  git remote set-url origin "$REMOTE_URL"
else
  git remote add origin "$REMOTE_URL"
fi

echo "==> status"
git status --short || true

echo "==> alles adden"
git add .

echo "==> commit machen falls noetig"
if git diff --cached --quiet; then
  echo "nichts neues zum committen"
else
  git commit -m "Initial commit"
fi

echo "==> push auf main"
git push -u origin main
EOF
