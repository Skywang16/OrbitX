#!/usr/bin/env bash
set -e

pnpm run build
# pnpm config get registry 
# pnpm config set registry=https://registry.npmjs.org

echo 'Login'
pnpm login

echo "Publish..."
pnpm publish --access=public

echo -e "\successful\n"
exit
