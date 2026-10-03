allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

val newBuildDir: Directory =
    rootProject.layout.buildDirectory
        .dir("../../build")
        .get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)
}
// Plugin modules ask for their own compileSdk, build-tools and NDK versions, which the
// read-only Nix Android SDK may not ship. Reuse what :app picked (see app/build.gradle.kts).
// Registered before :app is evaluated so it runs before AGP reads the plugin's settings.
subprojects {
    if (name != "app") {
        afterEvaluate {
            val app = rootProject.project(":app").extensions.getByType(com.android.build.gradle.BaseExtension::class.java)
            extensions.findByType(com.android.build.gradle.LibraryExtension::class.java)?.apply {
                app.compileSdkVersion?.let { compileSdkVersion(it) }
                buildToolsVersion = app.buildToolsVersion
                ndkVersion = app.ndkVersion
            }
        }
    }
}
subprojects {
    project.evaluationDependsOn(":app")
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}
