# BehaviorGuard consumer ProGuard rules.
# These are applied to apps that depend on the library (in addition to the app's own rules).

# Keep the JNI-bound class and all native methods.
-keep class com.behaviorgaurd.BehaviorGuard {
    private native <methods>;
    public <init>();
}

# Keep SessionOutcome sealed subclasses (used via `when` without reflection,
# but keep them in case consumers use reflection or Gson).
-keep class com.behaviorgaurd.SessionOutcome { *; }
-keep class com.behaviorgaurd.SessionOutcome$* { *; }

# Keep BehaviorGuardManager (convenience wrapper, may be instantiated by name).
-keep class com.behaviorgaurd.BehaviorGuardManager { *; }

# Prevent stripping of the native library load in the companion object.
-keepclassmembers class com.behaviorgaurd.BehaviorGuard {
    static <clinit>();
}
