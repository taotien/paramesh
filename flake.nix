{
    inputs = {
        flake-utils.url = "github:numtide/flake-utils";
    };
    outputs = {
        self,
        nixpkgs,
        flake-utils,
    }:
        flake-utils.lib.eachDefaultSystem (
            system: let
                pkgs = nixpkgs.legacyPackages.${system};
            in
                with pkgs; let
                    microcad = {};
                in {
                    devShell = pkgs.mkShell rec {
                        buildInputs = [
                            meshlab

                            just
                            maturin
                            python313

                            # stuff
                            pkg-config
                            ninja
                            cmake
                            stdenv.cc.cc

                            # rerun
                            nasm
                            wayland

                            # microcad-bevy
                            vulkan-loader
                            vulkan-tools
                            xorg.libX11
                            xorg.libXcursor
                            xorg.libXi
                            xorg.libXrandr
                            libxkbcommon
                        ];
                        LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
                    };
                }
        );
}
