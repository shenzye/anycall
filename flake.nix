{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default =
          with pkgs;
          mkShell {
            buildInputs = [
              # Rust
              (rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
                targets = [ "wasm32-unknown-unknown" ];
              })

              clippy
              lld
              rustfmt
              wasm-pack
              chromium
              chromedriver
              firefox
              geckodriver
              wasm-bindgen-cli_0_2_121
              llvmPackages.clang-unwrapped
              llvmPackages.llvm
            ];

            RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            # ring's C sources must be compiled to wasm objects, not host objects.
            CC_wasm32_unknown_unknown = "${llvmPackages.clang-unwrapped}/bin/clang";
            CFLAGS_wasm32_unknown_unknown = "--target=wasm32-unknown-unknown";
            AR_wasm32_unknown_unknown = "${llvmPackages.llvm}/bin/llvm-ar";

          };
      }
    );
}
