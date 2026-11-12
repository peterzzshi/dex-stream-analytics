plugins {
    kotlin("jvm") version "2.4.0"
    application
    id("org.jlleitschuh.gradle.ktlint") version "14.2.0"
    id("com.github.davidmc24.gradle.plugin.avro") version "1.9.1"
}

group = "com.web3analytics"
version = "0.1.0"

repositories {
    mavenCentral()
}

val ktorVersion = "3.5.2"
val logbackVersion = "1.6.3"
val jedisVersion = "5.1.0"
val jacksonVersion = "2.22.4"
val coroutinesVersion = "1.9.0"
val junitVersion = "5.11.4"
val ktlintVersion = "1.3.1"
val avroVersion = "1.12.2"

dependencies {
    implementation("io.ktor:ktor-server-core-jvm:$ktorVersion")
    implementation("io.ktor:ktor-server-netty-jvm:$ktorVersion")
    implementation("io.ktor:ktor-server-content-negotiation-jvm:$ktorVersion")
    implementation("io.ktor:ktor-server-status-pages-jvm:$ktorVersion")
    implementation("io.ktor:ktor-server-cors-jvm:$ktorVersion")
    implementation("io.ktor:ktor-server-websockets-jvm:$ktorVersion")
    implementation("io.ktor:ktor-server-html-builder-jvm:$ktorVersion")
    implementation("io.ktor:ktor-serialization-jackson-jvm:$ktorVersion")
    implementation("com.fasterxml.jackson.datatype:jackson-datatype-jsr310:$jacksonVersion")
    implementation("com.fasterxml.jackson.module:jackson-module-kotlin:$jacksonVersion")
    implementation("ch.qos.logback:logback-classic:$logbackVersion")
    implementation("redis.clients:jedis:$jedisVersion")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:$coroutinesVersion")
    implementation("org.apache.avro:avro:$avroVersion")

    testImplementation(kotlin("test"))
    testImplementation("io.ktor:ktor-server-test-host:$ktorVersion")
    testImplementation("io.ktor:ktor-client-content-negotiation:$ktorVersion")
    testImplementation("org.junit.jupiter:junit-jupiter:$junitVersion")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:$coroutinesVersion")
}

application {
    mainClass.set("com.web3analytics.alerting.ApplicationKt")
}

kotlin {
    jvmToolchain(21)
}

sourceSets {
    main {
        java {
            srcDir("build/generated-main-avro-java")
        }
    }
}

tasks.named("compileKotlin") {
    dependsOn("generateAvroJava")
}

tasks.named("compileTestKotlin") {
    dependsOn("generateTestAvroJava")
}

tasks.named("runKtlintCheckOverMainSourceSet") {
    dependsOn("generateAvroJava")
}

tasks.named("runKtlintCheckOverTestSourceSet") {
    dependsOn("generateTestAvroJava")
}

tasks.test {
    useJUnitPlatform()
}

avro {
    stringType.set("String")
    setOutputCharacterEncoding("UTF-8")
}

tasks.named<com.github.davidmc24.gradle.plugin.avro.GenerateAvroJavaTask>("generateAvroJava") {
    source("$projectDir/../schemas")
}

ktlint {
    version.set(ktlintVersion)
    android.set(false)
    ignoreFailures.set(false)
    reporters {
        reporter(org.jlleitschuh.gradle.ktlint.reporter.ReporterType.PLAIN)
        reporter(org.jlleitschuh.gradle.ktlint.reporter.ReporterType.CHECKSTYLE)
    }
    filter {
        exclude("**/generated/**")
        include("**/kotlin/**")
    }
}
