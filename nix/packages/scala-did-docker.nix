{
  dockerTools,
  pkgsInternal,
}:

dockerTools.buildLayeredImage {
  name = "scala-did";
  tag = "latest";
  contents = [
    pkgsInternal.scala-did
  ];
  config = {
    Env = [ ];
    Entrypoint = [ "/bin/scala-did-node" ];
    Cmd = [ ];
    WorkingDir = "/";
  };
}
