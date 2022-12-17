{
  description = "Floxpkgs/Project Template";
  nixConfig.bash-prompt = "[flox] \\[\\033[38;5;172m\\]λ \\[\\033[0m\\]";
  inputs.flox-floxpkgs.url = "github:flox/floxpkgs";

  # Declaration of external resources
  # =================================
  inputs.fenix = {
    url = "github:nix-community/fenix";
    inputs.nixpkgs.follows = "flox-floxpkgs/nixpkgs";
  };
  # =================================

  outputs = args @ {flox-floxpkgs, ...}: flox-floxpkgs.project args (_: {});
}
