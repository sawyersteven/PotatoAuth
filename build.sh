echo "Building for $OSTYPE"

if [ $OSTYPE == "msys" ]; then
    PUB_DIR="./build/windows"
    BIN="$HOME/.cargo/target/release/potato_auth.exe"
elif [ $OSTYPE == "linux-gnu" ]; then
    PUB_DIR="./build/linux" 
    BIN="$HOME/.cargo/target/release/potato_auth"
fi

echo $BIN

mkdir -p $PUB_DIR

cargo build -r

cp $BIN $PUB_DIR
cp -r "./static/" $PUB_DIR

echo "Build finished in $PUB_DIR"