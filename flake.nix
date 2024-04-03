{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    wfvm.url = "git+https://git.m-labs.hk/m-labs/wfvm";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, wfvm }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        x86 = system == "x86_64-linux";
        pkgs = import nixpkgs {
          inherit system overlays;
        } // (if x86 then {
            flaky-os = wfvm.lib.makeWindowsImage {
              windowsImage = "windows.iso";
            # configuration parameters go here
            };
        } else {});
        toolchain = pkgs.rust-bin.nightly.latest.default.override {
          extensions = ["rust-src" "clippy" "rust-analyzer" "miri"];
        };


        libraries = with pkgs;[
          webkitgtk_4_1
          gtk3
          cairo
          libxkbcommon
          pango
          libsoup_3
          gdk-pixbuf
          vulkan-loader
          glib
          dbus
          openssl_3
          librsvg
        ];

        packages = with pkgs; [
          curl
          wget
          pkg-config
          dbus
          openssl_3
          libxkbcommon
          vulkan-loader
          # cargo-tauri
          nodePackages.pnpm
          glib
          gtk3
          libsoup_3
          webkitgtk_4_1
          librsvg
          toolchain
        ];
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = packages ++ (if x86 then [pkgs.flaky-os] else []);

          shellHook =
            ''
              export WEBKIT_DISABLE_DMABUF_RENDERER=1
              export GDK_BACKEND="x11"
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
              export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
            '';
        };
      });
}

