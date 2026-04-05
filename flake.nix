{
  description = "Burrow development shell and forge host configuration";

  inputs = {
    nixpkgs.url = "tarball+https://codeload.github.com/NixOS/nixpkgs/tar.gz/nixos-unstable";
    flake-utils.url = "tarball+https://codeload.github.com/numtide/flake-utils/tar.gz/main";
    agenix = {
      url = "github:ryantm/agenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    disko = {
      url = "tarball+https://codeload.github.com/nix-community/disko/tar.gz/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nsc-autoscaler = {
      url = "git+https://compatible.systems/conrad/nsc-autoscaler.git";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    hcloud-upload-image-src = {
      url = "tarball+https://codeload.github.com/apricote/hcloud-upload-image/tar.gz/v1.3.0";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, agenix, disko, nsc-autoscaler, hcloud-upload-image-src }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
    in
    (flake-utils.lib.eachSystem supportedSystems (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        lib = pkgs.lib;
        commonPackages = with pkgs; [
          cargo
          rustc
          rustfmt
          clippy
          protobuf
          pkg-config
          sqlite
          git
          openssh
          curl
          jq
          nodejs_20
          python3
          rsync
        ];
        nscPkg =
          if pkgs.stdenv.isLinux || pkgs.stdenv.isDarwin then
            let
              version = "0.0.452";
              osName =
                if pkgs.stdenv.isLinux then
                  "linux"
                else if pkgs.stdenv.isDarwin then
                  "darwin"
                else
                  throw "nsc: unsupported host OS ${pkgs.stdenv.hostPlatform.system}";
              archInfo =
                if pkgs.stdenv.hostPlatform.isx86_64 then
                  {
                    arch = "amd64";
                    hash =
                      if pkgs.stdenv.isLinux then
                        "sha256-FBqOJ0UQWTv2r4HWMHrR/aqFzDa0ej/mS8dSoaCe6fY="
                      else
                        "sha256-3fRKWO0SCCa5PEym5yCB7dtyEx3xSxXSHfJYz8B+/4M=";
                  }
                else if pkgs.stdenv.hostPlatform.isAarch64 then
                  {
                    arch = "arm64";
                    hash =
                      if pkgs.stdenv.isLinux then
                        "sha256-A6twO8Ievbu7Gi5Hqon4ug5rCGOm/uHhlCya3px6+io="
                      else
                        "sha256-n363xLaGhy+a6lw2F+WicQYGXnGYnqRW8aTQCSppwcw=";
                  }
                else
                  throw "nsc: unsupported host platform ${pkgs.stdenv.hostPlatform.system}";
              src = pkgs.fetchurl {
                url = "https://github.com/namespacelabs/foundation/releases/download/v${version}/nsc_${version}_${osName}_${archInfo.arch}.tar.gz";
                sha256 = archInfo.hash;
              };
            in
            pkgs.stdenvNoCC.mkDerivation {
              pname = "nsc";
              inherit version src;
              dontConfigure = true;
              dontBuild = true;
              unpackPhase = ''
                tar -xzf "$src"
              '';
              installPhase = ''
                install -d "$out/bin"
                install -m 0555 nsc "$out/bin/nsc"
                install -m 0555 docker-credential-nsc "$out/bin/docker-credential-nsc"
                install -m 0555 bazel-credential-nsc "$out/bin/bazel-credential-nsc"
              '';
            }
          else
            null;
        hcloudUploadImagePkg = pkgs.buildGoModule {
          pname = "hcloud-upload-image";
          version = "1.3.0";
          src = hcloud-upload-image-src;
          vendorHash = "sha256-IdOAUBPg0CEuHd2rdc7jOlw0XtnAhr3PVPJbnFs2+x4=";
          subPackages = [ "." ];
          env.GOWORK = "off";
          ldflags = [
            "-s"
            "-w"
          ];
        };
        forgejoNscSrc = lib.cleanSourceWith {
          src = ./services/forgejo-nsc;
          filter = path: type:
            let
              p = toString path;
              name = builtins.baseNameOf path;
              hasDir = dir: lib.hasInfix "/${dir}/" p || lib.hasSuffix "/${dir}" p;
            in
            !(hasDir ".git" || hasDir "vendor" || hasDir "node_modules" || name == "result");
        };
        forgejoNscDispatcher = pkgs.buildGoModule {
          pname = "forgejo-nsc-dispatcher";
          version = "0.1.0";
          src = forgejoNscSrc;
          subPackages = [ "./cmd/forgejo-nsc-dispatcher" ];
          vendorHash = "sha256-Kpr+5Q7Dy4JiLuJVZbFeJAzLR7PLPYxhtJqfxMEytcs=";
        };
        forgejoNscAutoscaler = pkgs.buildGoModule {
          pname = "forgejo-nsc-autoscaler";
          version = "0.1.0";
          src = forgejoNscSrc;
          subPackages = [ "./cmd/forgejo-nsc-autoscaler" ];
          vendorHash = "sha256-Kpr+5Q7Dy4JiLuJVZbFeJAzLR7PLPYxhtJqfxMEytcs=";
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            commonPackages
            ++ [
              hcloudUploadImagePkg
              forgejoNscDispatcher
              forgejoNscAutoscaler
            ]
            ++ lib.optionals (nscPkg != null) [ nscPkg ];
        };

        devShells.ci = pkgs.mkShell {
          packages =
            commonPackages
            ++ [
              hcloudUploadImagePkg
            ]
            ++ lib.optionals (nscPkg != null) [ nscPkg ];
        };

        formatter = pkgs.nixpkgs-fmt;

        packages =
          {
            agenix = agenix.packages.${system}.agenix;
            hcloud-upload-image = hcloudUploadImagePkg;
            forgejo-nsc-dispatcher = forgejoNscDispatcher;
            forgejo-nsc-autoscaler = forgejoNscAutoscaler;
          }
          // lib.optionalAttrs (nscPkg != null) { nsc = nscPkg; };
      }))
    // {
      nixosModules.burrow-forge = import ./nixos/modules/burrow-forge.nix;
      nixosModules.burrow-forge-runner = import ./nixos/modules/burrow-forge-runner.nix;
      nixosModules.burrow-forgejo-nsc = nsc-autoscaler.nixosModules.default;
      nixosModules.burrow-authentik = import ./nixos/modules/burrow-authentik.nix;
      nixosModules.burrow-headscale = import ./nixos/modules/burrow-headscale.nix;

      nixosConfigurations.burrow-forge = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = {
          inherit self;
        };
        modules = [
          agenix.nixosModules.default
          disko.nixosModules.disko
          ./nixos/hosts/burrow-forge/default.nix
        ];
      };

      images = {
        burrow-forge-raw = self.nixosConfigurations.burrow-forge.config.system.build.diskoImages;
      };
    };
}
