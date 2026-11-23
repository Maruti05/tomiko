# Tomiko production rules.
# Only manifest entry points and JNI entry points must survive obfuscation.

-keepattributes SourceFile,LineNumberTable
-keep class com.vedica.labs.ind.app.talkin.tomiko.MainActivity { *; }
-keep class com.vedica.labs.ind.app.talkin.tomiko.TomikoApplication { *; }
-keep class com.vedica.labs.ind.app.talkin.tomiko.game.CharacterGameActivity { *; }
-keep class com.vedica.labs.ind.app.talkin.tomiko.native.NativeEngine { *; }

# JNI methods are resolved by name from Rust.
-keepclasseswithmembernames,includedescriptorclasses class * {
    native <methods>;
}

# Kotlin serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** {
    *** Companion;
}
-keepclasseswithmembers class kotlinx.serialization.json.** {
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class com.vedica.labs.ind.app.talkin.tomiko.**$$serializer { *; }
-keepclassmembers class com.vedica.labs.ind.app.talkin.tomiko.** {
    *** Companion;
}
-keepclasseswithmembers class com.vedica.labs.ind.app.talkin.tomiko.** {
    kotlinx.serialization.KSerializer serializer(...);
}
