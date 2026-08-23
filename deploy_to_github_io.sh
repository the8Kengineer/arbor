cd www &&
rm -rf dist/* &&
trunk build --release &&
cd .. &&
rm -rf docs/*
cp -R www/dist/* docs/ &&
sed -i '' -e 's#/index#./index#g; s#/www#./www#g' ./docs/index.html
git add docs/*