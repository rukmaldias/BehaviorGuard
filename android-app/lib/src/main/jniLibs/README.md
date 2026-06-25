# jniLibs

Place the compiled native libraries here before building the AAR.

Run from the repo root:

```sh
./build-android.sh
```

Or manually:

```sh
cargo ndk \
    -t arm64-v8a \
    -t armeabi-v7a \
    -t x86_64 \
    -o android-app/lib/src/main/jniLibs \
    build --release --features jni
```

Expected files after build:
- `arm64-v8a/libbehavior_guard.so`
- `armeabi-v7a/libbehavior_guard.so`
- `x86_64/libbehavior_guard.so`

The `.so` files are excluded from git (see `.gitignore`).
