{
  description = "Vitrum development environment";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      nightlyToolchain = pkgs.rust-bin.nightly.latest.minimal.override {
        extensions = ["rustfmt"];
      };
    in {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          python3
          ruff
          dprint
          nixfmt
          rustToolchain
          nightlyToolchain
          pkg-config
          cmake
          clang
          llvmPackages.libclang
          llvmPackages.libclang.lib
          llvmPackages.bintools
          gnumake
          wayland
          wayland-protocols
          wayland-scanner
          libxkbcommon
          libinput
          pixman
          libgbm
          mesa
          libglvnd
          libdrm
          vulkan-loader
          vulkan-headers
          seatd
          dbus
          xwayland
          perf
          valgrind
          heaptrack
          flamegraph
          rust-analyzer
        ];
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
          pkgs.llvmPackages.libclang.lib
          pkgs.wayland
          pkgs.libxkbcommon
          pkgs.libinput
          pkgs.mesa
          pkgs.libglvnd
          pkgs.libdrm
          pkgs.vulkan-loader
          pkgs.seatd
          pkgs.dbus
        ];
        WINIT_UNIX_BACKEND = "wayland";
        WLR_RENDERER = "vulkan";
        RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        RUSTFMT = "${nightlyToolchain}/bin/rustfmt";
        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        shellHook = ''
          export PATH="$PWD/.cargo/bin:$PATH"
          echo "Vitrum dev shell ready."
          echo "Profilers available: flamegraph, heaptrack, valgrind (massif/callgrind/helgrind), perf"
        '';
      };
    });
}