{
  config,
  lib,
  pkgs,
  ...
}:
with lib; let
  format = pkgs.formats.toml {};
  configFile = format.generate "demostf-frontend.toml" {
    output.target = cfg.outputPath;
    mqtt = {
      inherit (cfg.mqtt) hostname port username;
      "password-file" = "$CREDENTIALS_DIRECTORY/mqtt_password";
    };
    device."password-file" = "$CREDENTIALS_DIRECTORY/device_password";
  };
  cfg = config.services.demostf-frontend;
in {
  options.services.demostf-frontend = {
    enable = mkEnableOption "Log archiver";

    outputPath = mkOption {
      type = types.str;
      description = "Directory to save the backups into";
    };

    mqtt = mkOption {
      type = types.submodule {
        options = {
          hostname = mkOption {
            type = types.str;
            description = "MQTT hostname";
          };
          port = mkOption {
            type = types.port;
            default = 1883;
            description = "MQTT port";
          };
          username = mkOption {
            type = types.str;
            description = "MQTT username";
          };
          passwordFile = mkOption {
            type = types.str;
            description = "File containing the MQTT password";
          };
        };
      };
      description = "MQTT options";
    };

    devicePasswordFile = mkOption {
      type = types.str;
      description = "File containing the device password";
    };

    interval = mkOption {
      type = types.str;
      default = "daily";
      description = "Interval to run the backup";
    };

    package = mkOption {
      type = types.package;
      defaultText = literalExpression "pkgs.tasproxy";
      description = "package to use";
    };
  };

  config = mkIf cfg.enable {
    systemd.services."demostf-frontend" = {
      description = "Backup tasmota configurations";

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/demostf-frontend ${configFile}";
        LoadCredential = [
          "mqtt_password:${cfg.mqtt.passwordFile}"
          "device_password:${cfg.devicePasswordFile}"
        ];
        ReadWritePaths = [cfg.outputPath];
        Restart = "on-failure";
        DynamicUser = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        NoNewPrivileges = true;
        PrivateDevices = true;
        ProtectClock = true;
        CapabilityBoundingSet = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
        SystemCallArchitectures = "native";
        ProtectKernelModules = true;
        RestrictNamespaces = true;
        MemoryDenyWriteExecute = true;
        ProtectHostname = true;
        LockPersonality = true;
        ProtectKernelTunables = true;
        RestrictAddressFamilies = "AF_INET AF_INET6";
        RestrictRealtime = true;
        ProtectProc = "noaccess";
        SystemCallFilter = ["@system-service" "~@resources" "~@privileged"];
        IPAddressDeny = "multicast";
        PrivateUsers = true;
        ProcSubset = "pid";
        RuntimeDirectory = "demostf-frontend";
        RestrictSUIDSGID = true;
      };
    };

    systemd.timers."demostf-frontend" = {
      inherit (config.systemd.services."demostf-frontend") description;

      enable = true;
      wantedBy = ["multi-user.target"];
      timerConfig = {
        OnCalendar = cfg.interval;
        RandomizedDelaySec = "15m";
      };
    };
  };
}