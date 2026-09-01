{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.rulmee;

  dmcfg = config.services.displayManager;
  desktops = dmcfg.sessionData.desktops;

  version = "2.0.1";
  rulmeePkg = pkgs.callPackage ./rulmee.nix {
    inherit pkgs;
    config = {
      inherit version lib;
      cfg = cfg.config;
      src = pkgs.fetchFromGitHub {
        owner = "BeniaminK";
        repo = "rulmee";
        rev = "v${version}";
        sha256 = "sha256-bpUqhD1JSiYRf7w7ylEMXHMvEpnSri1zZSxRQPdZWB4=";
      };

      xsessions = "${desktops}/share/xsessions";
      wayland-sessions = "${desktops}/share/wayland-sessions";
    };
  };
in
{
  options = {
    rulmee.keysEnum = lib.mkOption {
      type = with lib.types; attrs;
      default = rulmee.passthru.keysEnum;
      readOnly = true;
      description = "Keys enum constants";
    };
    services.rulmee.config = lib.mkOption {
      type =
        with lib.types;
        oneOf [
          str
          attrs
        ];
      default = { };
      description = "Config options for rulmee | Either attr tree or name of bundled themes";
    };
  };
  config = {
    services.displayManager.defaultSession = "rulmee";

    systemd.services.rulmee = {
      description = "TUI display manager";
      aliases = [ "display-manager.service" ];
      after = [
        "systemd-user-sessions.service"
        "plymouth-quit-wait.service"
      ];
      serviceConfig = {
        Type = "idle";
        ExecStart = "${rulmeePkg}/bin/rulmee 7";
        StandardInput = "tty";
        StandardOutput = "tty";
        StandardError = "tty";
        TTYPath = "/dev/tty7";
        TTYReset = "yes";
        TTYVHangup = "yes";
      };
    };
  };
}
