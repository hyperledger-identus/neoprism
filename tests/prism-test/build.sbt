val scala3Version = "3.3.8"

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
