#!/usr/bin/env bash
set -e

# VERSION=$(grep -o '"version": *"[^"]*"' package.json | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+')
# sed -i '' "s/\"workspace:\\*\"/\"^$VERSION\"/g" package.json

pnpm run build
# pnpm config get registry 
# pnpm config set registry=https://registry.npmjs.org

echo 'Login'
pnpm login

echo "Publish..."
pnpm publish --access=public --no-git-checks

echo -e "\successful\n"
exit
