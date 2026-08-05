# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# Preserve line numbers for debugging stack traces
-keepattributes SourceFile,LineNumberTable
-renamesourcefileattribute SourceFile

# === JNI native methods ===
# Keep all classes with native methods — the native side looks up
# method names by reflection via JNIEnv.GetMethodID / GetFieldID.
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep the Renderer class — it's called from JNI
-keep class io.twoyi.Renderer { *; }
-keep class io.twoyi.Renderer$* { *; }

# === Reflection ===
# FreeReflection library uses hidden API reflection
-keep class com.jaredzrr.navigationbarhide.** { *; }

# libsu shell utilities
-keep class com.topjohnwu.superuser.** { *; }

# === Glide ===
-keep public class * implements com.bumptech.glide.module.GlideModule
-keep class * extends com.bumptech.glide.module.AppGlideModule { <init>(...); }
-keep class com.bumptech.glide.load.data.ParcelFileDescriptorRewinder$InternalRewinder { *** rewind(); }

# === DocumentsProvider ===
-keep class io.twoyi.provider.TwoyiDocumentsProvider { *; }

# === JDeferred ===
-keep class org.jdeferred.** { *; }
-keepclassmembers class org.jdeferred.** { *; }

# === Material Dialogs ===
-keep class com.afollestad.materialdialogs.** { *; }

# === Keep enum values ===
-keepclassmembers enum * {
    public static **[] values();
    public static ** valueOf(java.lang.String);
}
