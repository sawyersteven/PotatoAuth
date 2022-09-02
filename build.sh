echo "Read comments in this script before running or linux may not build correctly"
# Running commands in wsl directly (eg `wsl -e foo`) tends to fail if the binary
# in question is not in /usr/bin/. Other locations may work, but this worked for
# me. So make a link to cargo `sudo ln -s /home/USER/.cargo/bin/cargo /usr/bin/`
# so it knows where to find cargo.

read -n 1 -r -s -p "Press any key to continue..." key

echo "Building PotatoAuth"
rm -rf ./build/
mkdir -p ./build/windows/
mkdir -p ./build/linux/

echo "🐧 Building for linux via wsl"
wsl -e cargo build -r
echo "🪟 Building for windows"
cargo build -r

echo "Copying bundled files to ./build/<target>/"

cp $HOME/.cargo/target/release/potato_auth.exe ./build/windows/
cp -r ./static/ ./build/windows/
cp ./potato_auth.nginx.conf ./build/windows/

cp $HOME/.cargo/target/release/potato_auth ./build/linux/
cp -r ./static/ ./build/linux/
cp ./potato_auth.nginx.conf ./build/linux/

echo "Done"