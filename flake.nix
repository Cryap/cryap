{
  description = "Cryap";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    flake-compat.url = "https://flakehub.com/f/edolstra/flake-compat/1.tar.gz";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    rust-overlay,
    flake-utils,
    advisory-db,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          (import rust-overlay)
        ];
      };

      toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
        toolchain.default.override {
          extensions = ["rust-src"];
          targets = ["wasm32-unknown-unknown"];
        });

      inherit (pkgs) lib;

      craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
      src = craneLib.cleanCargoSource (craneLib.path ./.);

      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        pname = "cryap";
        version = "0.0.1";
        buildInputs =
          [
            pkgs.postgresql
            pkgs.yarn
            # Add additional build inputs here
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

        # Additional environment variables can be set directly
        # MY_CUSTOM_VAR = "some value";
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      my-crate = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
        });
    in {
      checks =
        {
          # Build the crate as part of `nix flake check` for convenience
          inherit my-crate;

          # Run clippy (and deny all warnings) on the crate source,
          # again, resuing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          my-crate-clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            });

          my-crate-doc = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
            });

          # Check formatting
          my-crate-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          my-crate-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `my-crate` if you do not want
          # the tests to run twice
          my-crate-nextest = craneLib.cargoNextest (commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });
        }
        // lib.optionalAttrs (system == "x86_64-linux") {
          # NB: cargo-tarpaulin only supports x86_64 systems
          # Check code coverage (note: this will not upload coverage anywhere)
          my-crate-coverage = craneLib.cargoTarpaulin (commonArgs
            // {
              inherit cargoArtifacts;
            });
        };

      packages = {
        default = my-crate;
      };

      apps.default = flake-utils.lib.mkApp {
        drv = my-crate;
      };

      devShells.default = craneLib.devShell {
        checks = self.checks.${system};

        shellHook = ''
          export FRONTEND_DIST=$(pwd)/crates/frontend/dist
        '';

        # Extra inputs can be added here
        packages = with pkgs; [
          just
          openssl
          pkg-config
          yarn
          trunk
          taplo
          python313Packages.mkdocs-material
          python313Packages.mkdocs
        ];
      };

      formatter = pkgs.alejandra;
    });
}
