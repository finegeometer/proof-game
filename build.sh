cp ../proof-game/pkg/* pkg/ -r
cp ../proof-game/index.html . -r
cp ../proof-game/levels.json . -r
git add .
git commit -m "Update GH pages."
git push

