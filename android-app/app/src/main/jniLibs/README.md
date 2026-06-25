# jniLibs

Place the compiled `.so` files here before building the Android project.

Build from the repository root:

```sh
cargo ndk \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o android-app/app/src/main/jniLibs \
  build --release --features jni
```

Expected layout after build:

```
jniLibs/
├── arm64-v8a/
│   └── libbehavior_guard.so
├── armeabi-v7a/
│   └── libbehavior_guard.so
└── x86_64/
    └── libbehavior_guard.so
```
