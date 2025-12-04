val scala3Version = "3.3.7"

val V = new {
  val zio = "2.1.23"
  val zioHttp = "3.7.1"
  val monocle = "3.3.0"
  val apollo = "1.8.0"
  val grpcNetty = "1.77.0"
}

val D = new {
  val scalaPbDeps: Seq[ModuleID] = Seq(
    "com.thesamet.scalapb" %% "scalapb-runtime" % scalapb.compiler.Version.scalapbVersion % "protobuf",
    "com.thesamet.scalapb" %% "scalapb-runtime-grpc" % scalapb.compiler.Version.scalapbVersion
  )

  val apolloDeps: Seq[ModuleID] = Seq(
    "org.hyperledger.identus" % "apollo-jvm" % V.apollo exclude (
      "net.jcip",
      "jcip-annotations"
    ), // Exclude because of license
    "com.github.stephenc.jcip" % "jcip-annotations" % "1.0-1" % Runtime // Replace for net.jcip % jcip-annotations"
  )

  val deps: Seq[ModuleID] = Seq(
    "dev.zio" %% "zio" % V.zio,
    "io.grpc" % "grpc-netty-shaded" % V.grpcNetty,
    "dev.zio" %% "zio-http" % V.zioHttp,
    "dev.optics" %% "monocle-core" % V.monocle,
    "dev.optics" %% "monocle-macro" % V.monocle
  )

  val testDeps: Seq[ModuleID] = Seq(
    "dev.zio" %% "zio-test" % V.zio % Test,
    "dev.zio" %% "zio-test-sbt" % V.zio % Test,
    "dev.zio" %% "zio-test-magnolia" % V.zio % Test
  )
}

lazy val root = project
  .in(file("."))
  .settings(
    name := "prism-test",
    version := "0.1.0-SNAPSHOT",
    scalaVersion := scala3Version,
    scalacOptions := Seq(
      "-feature",
      "-deprecation",
      "-unchecked",
      "-Wunused:all"
    ),
    testFrameworks += new TestFramework("zio.test.sbt.ZTestFramework"),
    Compile / PB.targets := Seq(
      scalapb.gen() -> (Compile / sourceManaged).value / "scalapb"
    ),
    Compile / PB.protoSources := Seq(
      baseDirectory.value / ".." / ".." / "lib" / "did-prism" / "proto",
      (Compile / resourceDirectory).value // includes scalapb codegen package wide config
    ),
    libraryDependencies ++= D.scalaPbDeps ++ D.apolloDeps ++ D.deps ++ D.testDeps
  )
