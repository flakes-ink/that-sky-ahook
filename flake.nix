{
  description = "Dev Shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }@inputs:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        };
        ndkZip = pkgs.androidenv.androidPkgs.all.packages.ndk.v28_2_13676358;

        ndk = pkgs.stdenv.mkDerivation {
          name = "android-ndk-unpacked";
          src = ndkZip;
          nativeBuildInputs = [
            pkgs.unzip
            pkgs.autoPatchelfHook
          ];
          buildInputs = [
            pkgs.stdenv.cc.cc.lib
            pkgs.zlib
          ];
          phases = [
            "unpackPhase"
            "installPhase"
            "fixupPhase"
          ];
          unpackPhase = ''
            unzip $src
            mv android-ndk-* ndk
          '';
          installPhase = ''
            mkdir -p $out
            cp -r ndk $out/ndk
          '';
          postFixup = ''
            patchShebangs $out
          '';
          autoPatchelfIgnoreMissingDeps = [
            "libbz2.so.1"
            "libncursesw.so.5"
            "libtinfo.so.5"
            "libpanelw.so.5"
            "libcrypt.so.1"
          ];
        };

        # Pristine Lua 5.2.0 built as a static library for the Android
        # target. mlua's `lua52` feature is pointed at this exact release
        # (5.2.0) instead of the 5.2.4 that `lua-src` would vendor.
        lua520 = pkgs.stdenv.mkDerivation {
          name = "lua-5.2.0-aarch64-linux-android";
          src = pkgs.fetchurl {
            url = "https://www.lua.org/ftp/lua-5.2.0.tar.gz";
            sha256 = "cabe379465aa8e388988073d59b69e76ba0025429d2c1da80821a252cdf6be0d";
          };
          nativeBuildInputs = [ ndk ];
          dontConfigure = true;
          buildPhase = ''
            runHook preBuild
            src_root="$PWD"
            cc="${ndk}/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"
            ar="${ndk}/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            cd "$src_root/src"
            for f in *.c; do
              case "$(basename "$f")" in
                lua.c | luac.c) continue ;; # REPL/compiler mains, not in liblua.a
              esac
              "$cc" -O2 -fPIC -DLUA_USE_POSIX -I. -c "$f"
            done
            mkdir -p "$out/lib"
            "$ar" rcs "$out/lib/liblua.a" *.o
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            mkdir -p "$out/include"
            cp "$src_root/src/lua.h" "$src_root/src/luaconf.h" "$src_root/src/lualib.h" "$src_root/src/lauxlib.h" "$src_root/src/lua.hpp" "$out/include/"
            runHook postInstall
          '';
        };
        env = rec {
          CC_aarch64_linux_android = (
            ndk + "/ndk/toolchains/llvm/prebuilt/linux-x86_64" + "/bin/aarch64-linux-android21-clang"
          );
          CXX_aarch64_linux_android = (
            ndk + "/ndk/toolchains/llvm/prebuilt/linux-x86_64" + "/bin/aarch64-linux-android21-clang++"
          );
          AR_aarch64_linux_android = (ndk + "/ndk/toolchains/llvm/prebuilt/linux-x86_64" + "/bin/llvm-ar");
          CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = CC_aarch64_linux_android;
          # mlua (`lua52`, non-vendored): link the exact Lua 5.2.0 build above
          # instead of the 5.2.4 that lua-src would otherwise vendor.
          LUA_LIB = "${lua520}/lib";
          LUA_LIB_NAME = "lua";
          LUA_LINK = "static";
        };
        rustToolchain = pkgs.rust-bin.beta.latest.default.override {
          targets = [
            "aarch64-linux-android"
          ];
        };

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
        packages = craneLib.buildPackage {
          nativeBuildInputs = [
            ndk
          ];

          inherit env;
          src = craneLib.cleanCargoSource ./.;
          cargoExtraArgs = "--target aarch64-linux-android";
          doCheck = false;
        };

      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            inherit env;
            buildInputs = [
              (pkgs.rust-bin.beta.latest.default.override {
                extensions = [
                  "rust-src"
                  "rust-analyzer"
                  "clippy"
                ];
                targets = [
                  "aarch64-linux-android"
                ];
              })
              package-version-server
            ];
            shellHook = "";
          };

        packages.default = packages;

        hydraJobs = {
          x86_64-linux = {
            default = pkgs.runCommand "that-sky-ahook-so" { } ''
              mkdir -p $out
              cp ${packages}/lib/libthat_sky_ahook.so $out/
              mkdir -p $out/nix-support
              echo "file binary-dist $out/libthat_sky_ahook.so" > $out/nix-support/hydra-build-products
            '';
          };
        };
      }
    );
}
