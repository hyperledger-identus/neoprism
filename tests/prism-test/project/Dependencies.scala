import sbt.*

object V {
  val zio = "2.1.26"
  val zioHttp = "3.11.3"
  val monocle = "3.3.0"
  val apollo = "1.8.8"
  val grpcNetty = "1.83.0"
}

object D {
  val scalaPbDeps: Seq[ModuleID] = Seq(
    "com.thesamet.scalapb" %% "scalapb-runtime" % scalapb.compiler.Version.scalapbVersion % "protobuf",
    "com.thesamet.scalapb" %% "scalapb-runtime-grpc" % scalapb.compiler.Version.scalapbVersion
  )

  val apolloDeps: Seq[ModuleID] = Seq(
    ("org.hyperledger.identus" % "apollo-jvm" % V.apollo).exclude(
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
