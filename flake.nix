{
  description = "A ✨fully✨ colorful customizable TUI display manager written in Rust.";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs =
    {
      flake-utils,
      nixpkgs,
      self,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        name = "rulmee";
        version = builtins.elemAt (builtins.match "VERSION[[:blank:]]*=[[:space:]]*([^\n]*)\n.*" (builtins.readFile ./Makefile)) 0;

        rulmee = pkgs.callPackage assets/pkg/nix/rulmee.nix {
          inherit pkgs;
          lib = pkgs.lib;
          config = {
            inherit version;
            src = ./.;
            xsessions = null;
            wayland-sessions = null;
            cfg = null;
            # cfg = "cherry";
          };
        };
      in
      rec {
        defaultApp = flake-utils.lib.mkApp { drv = defaultPackage; };
        defaultPackage = rulmee;
        devShell = pkgs.mkShell { buildInputs = rulmee.nativeBuildInputs ++ [ pkgs.clang-tools ]; };
        formatter = nixpkgs.legacyPackages.${system}.nixfmt-tree;
      }
    )
    // {
      nixosModules.rulmee = assets/pkg/nix/module.nix;
    };
}
