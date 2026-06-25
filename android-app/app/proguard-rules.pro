# Keep the BehaviorGuard Kotlin wrapper so R8 does not rename it.
# The JNI function names inside libbehavior_guard.so are hardcoded as
# Java_com_example_behaviorgaurd_BehaviorGuard_* — if this class is renamed,
# every native call throws UnsatisfiedLinkError at runtime.
-keep class com.example.behaviorgaurd.BehaviorGuard { *; }
-keep class com.example.behaviorgaurd.SessionOutcome { *; }
-keep class com.example.behaviorgaurd.SessionOutcome$* { *; }
