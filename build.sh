echo "Building PotatoAuth"
rm -r ./build/
mkdir -p ./build/windows/
mkdir -p ./build/linux/

# for some reason wsl doesn't understand `cargo`, but it understands calling
# literally any other command, then `cargo`, so just send a null `:` first
echo "Building for linux on wsl"
wsl -- :; cargo build -r;
echo "Building for windows"
cargo build -r

echo "Copying to ./build/<target>/"

cp $HOME/.cargo/target/release/potato_auth.exe ./build/windows/
cp -r ./static/ ./build/windows/
cp ./potato_auth.nginx.conf ./build/windows/

cp $HOME/.cargo/target/release/potato_auth ./build/linux/
cp -r ./static/ ./build/linux/
cp ./potato_auth.nginx.conf ./build/linux/
