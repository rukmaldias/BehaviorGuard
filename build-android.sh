#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-android.sh — Build libbehavior_guard.so for all ABIs and produce the
#                    behavior-guard AAR.
#
# Prerequisites:
#   • Rust + cargo  (https://rustup.rs)
#   • cargo-ndk     (cargo install cargo-ndk)
#   • Android NDK   (set ANDROID_NDK_HOME or let cargo-ndk find it)
#   • Android SDK + Gradle 8.9 (set ANDROID_HOME)
#
# Usage:
#   ./build-android.sh [--release|--debug] [--publish-local]
#
#   --release        Build Rust in release mode (default)
#   --debug          Build Rust in debug mode
#   --publish-local  After building, publish AAR to ~/.m2/repository
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_PROFILE="release"
PUBLISH_LOCAL=false

for arg in "$@"; do
  case "$arg" in
    --debug)         CARGO_PROFILE="debug" ;;
    --release)       CARGO_PROFILE="release" ;;
    --publish-local) PUBLISH_LOCAL=true ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

JNI_LIBS_DIR="$SCRIPT_DIR/android-app/lib/src/main/jniLibs"
TARGETS=("arm64-v8a" "armeabi-v7a" "x86_64")

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  BehaviorGuard — Android native build"
echo "  Profile  : $CARGO_PROFILE"
echo "  ABIs     : ${TARGETS[*]}"
echo "  Output   : $JNI_LIBS_DIR"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

cd "$SCRIPT_DIR"

# ── 1. Build native .so for all ABIs ─────────────────────────────────────────

CARGO_FLAGS="--features jni"
if [ "$CARGO_PROFILE" = "release" ]; then
  CARGO_FLAGS="$CARGO_FLAGS --release"
fi

cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o "$JNI_LIBS_DIR" \
  build $CARGO_FLAGS

echo ""
echo "✓ Native libraries built:"
for abi in "${TARGETS[@]}"; do
  so="$JNI_LIBS_DIR/$abi/libbehavior_guard.so"
  if [ -f "$so" ]; then
    size=$(du -h "$so" | cut -f1)
    echo "    $abi/libbehavior_guard.so  ($size)"
  else
    echo "    WARNING: $so not found"
  fi
done

# ── 2. Assemble the AAR ───────────────────────────────────────────────────────

echo ""
echo "Assembling AAR..."
cd "$SCRIPT_DIR/android-app"
./gradlew :lib:assembleRelease --quiet

AAR_PATH=$(find lib/build/outputs/aar -name "*release*.aar" 2>/dev/null | head -1)
if [ -n "$AAR_PATH" ]; then
  size=$(du -h "$AAR_PATH" | cut -f1)
  echo "✓ AAR built: $AAR_PATH  ($size)"
else
  echo "WARNING: AAR file not found in lib/build/outputs/aar/"
fi

# ── 3. Optionally publish to Maven Local ─────────────────────────────────────

if [ "$PUBLISH_LOCAL" = true ]; then
  echo ""
  echo "Publishing to Maven Local (~/.m2/repository)..."
  ./gradlew :lib:publishToMavenLocal --quiet
  echo "✓ Published: com.behaviorgaurd:behavior-guard:0.1.0"
  echo ""
  echo "Add to your app's build.gradle.kts:"
  echo "    repositories { mavenLocal() }"
  echo "    dependencies {"
  echo "        implementation(\"com.behaviorgaurd:behavior-guard:0.1.0\")"
  echo "    }"
fi

echo ""
echo "Done."
