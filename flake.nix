{
  description = "faqe Rust development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?rev=4c1018dae018162ec878d42fec712642d214fdfa";
    flake-utils.url = "github:numtide/flake-utils";
    nixgl.url = "github:nix-community/nixGL";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, flake-utils, nixgl, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          (import rust-overlay)
          (final: prev: {
            xorg = prev.xorg // {
              libX11 = final.libx11;
              libxcb = final.libxcb;
              libxshmfence = final.libxshmfence;
            };
          })
        ];

        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
            nvidia.acceptLicense = true;
          };
        };

        nvidiaVersion = builtins.getEnv "NVIDIA_VERSION";
        hasNvidia = nvidiaVersion != "";

        nixglPkgs = import "${nixgl}/default.nix" ({
          inherit pkgs;
        } // pkgs.lib.optionalAttrs hasNvidia {
          inherit nvidiaVersion;
          nvidiaHash = null;
        });

        nixGLTarget =
          if hasNvidia
          then "${nixglPkgs.nixGLNvidia}/bin/nixGLNvidia-${nvidiaVersion}"
          else "${nixglPkgs.nixGLIntel}/bin/nixGLIntel";
        nixVulkanTarget =
          if hasNvidia
          then "${nixglPkgs.nixVulkanNvidia}/bin/nixVulkanNvidia-${nvidiaVersion}"
          else "${nixglPkgs.nixVulkanIntel}/bin/nixVulkanIntel";

        nixGLAlias = pkgs.runCommand "nixGL" { } ''
          mkdir -p $out/bin
          ln -s ${nixGLTarget} $out/bin/nixGL
        '';
        nixVulkanAlias = pkgs.runCommand "nixVulkan" { } ''
          mkdir -p $out/bin
          ln -s ${nixVulkanTarget} $out/bin/nixVulkan
        '';

        guiLibs = with pkgs; [
          alsa-lib
          udev
          vulkan-loader
          libxkbcommon
          wayland
          libx11
          libxcursor
          libxi
          libxrandr
        ];
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform { cargo = rustToolchain; rustc = rustToolchain; };

        # The release binary, as `make release-build`: the web app built for wasm32,
        # bound and shrunk, then embedded into the CLI.
        faqe = rustPlatform.buildRustPackage {
          pname = "faqe";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./Cargo.toml ./Cargo.lock ./crates ./LICENSE ./LICENSES ./THIRD_PARTY.md
            ];
          };
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.wasm-bindgen-cli pkgs.binaryen ];
          cargoBuildFlags = [ "-p" "faqe-cli" ];
          preBuild = ''
            export FAQE_EMBED_DIR=$NIX_BUILD_TOP/faqe-embedded
            mkdir -p "$FAQE_EMBED_DIR"
            cargo build -p faqe-web --target wasm32-unknown-unknown --release --offline
            wasm-bindgen target/wasm32-unknown-unknown/release/faqe_web.wasm --target web \
              --out-dir "$FAQE_EMBED_DIR" --out-name faqe_web
            wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
              "$FAQE_EMBED_DIR/faqe_web_bg.wasm" -o "$FAQE_EMBED_DIR/faqe_web_bg.wasm.opt"
            mv "$FAQE_EMBED_DIR/faqe_web_bg.wasm.opt" "$FAQE_EMBED_DIR/faqe_web_bg.wasm"
            cp LICENSE THIRD_PARTY.md "$FAQE_EMBED_DIR/"
            cp -R LICENSES "$FAQE_EMBED_DIR/licenses"
          '';
          # The tests read the content submodule, which builds don't check out.
          doCheck = false;
          meta = { description = "One-binary Markdown-to-WASM personal website generator"; mainProgram = "faqe"; };
        };
      in
      {
        packages = { inherit faqe; default = faqe; };

        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.rust-analyzer
            pkgs.git-cliff
            pkgs.clang
            pkgs.lld
            pkgs.trunk
            pkgs.wasm-bindgen-cli
            pkgs.binaryen
            pkgs.curl
            pkgs.dart-sass
            pkgs.mold
            pkgs.pkg-config
            pkgs.presenterm

            nixGLAlias
            nixVulkanAlias
            nixglPkgs.nixGLIntel
            nixglPkgs.nixVulkanIntel
          ] ++ pkgs.lib.optionals hasNvidia [
            nixglPkgs.nixGLNvidia
            nixglPkgs.nixVulkanNvidia
          ] ++ guiLibs;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath guiLibs;
          WGPU_VALIDATION = "0";
          WGPU_DEBUG = "0";
        };
      }
    );
}
