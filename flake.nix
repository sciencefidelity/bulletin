{
  description = "A Rust devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        check = pkgs.writeShellScriptBin "check" ''
          cargo clippy --tests
        '';
        run = pkgs.writeShellScriptBin "run" ''
          cargo run
        '';
        test = pkgs.writeShellScriptBin "test" ''
          cargo test
        '';
        watch = pkgs.writeShellScriptBin "watch" ''
          ${pkgs.watchexec}/bin/watchexec -e rs -r cargo run
        '';
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            check
            pkg-config
            postgresql_17_jit
            sqlx-cli
            run
            taplo
            test
            watch
            (import ./scripts/init.nix { inherit pkgs; })
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-analyzer" "rust-src" ];
            })
          ];

          shellHook = /*bash*/ ''
          ''
          # enable mold linker for Linux
          + pkgs.lib.optionalString pkgs.stdenv.isLinux /*bash*/ ''
            export RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=${pkgs.mold-wrapped}/bin/mold"
          '';
        };
      }
    );
}

