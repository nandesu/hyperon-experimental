#!/bin/sh

# Default values for variables
HYPERONC_URL="https://github.com/nandesu/hyperonc-experimental.git"
HYPERONC_REV="modules"

# Function to parse command-line arguments
parse_args() {
    while getopts 'u:r:' opt; do
        case "$opt" in
            u) HYPERONC_URL="$OPTARG" ;;
            r) HYPERONC_REV="$OPTARG" ;;
            ?|h)
                echo "Usage: $(basename $0) [-u hyperonc_repo_url] [-r hyperonc_revision]"
                exit 1
                ;;
        esac
    done
}

# Function to install dependencies for manylinux2014
install_manylinux_deps() {
    if [ "$AUDITWHEEL_POLICY" = "manylinux2014" ]; then
        yum install -y python3 perl-devel openssl-devel pkgconfig cpan
    fi
}

# Function to install Rust and cbindgen
install_rust_cbindgen() {
    echo "==============================="
    echo "Phase 1: Cargo install cbindgen"
    echo "==============================="
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup.sh
    sh /tmp/rustup.sh -y && rm /tmp/rustup.sh
    export PATH="${PATH}:${HOME}/.cargo/bin"
    cargo install cbindgen
}

# Function to set up Perl and Python environments
setup_perl_python_env() {
    echo "==============================="
    echo "Phase 2: Conant -- Perl, Python, and pip23.1.2"
    echo "==============================="

    # PERL stuff
    export PERL_MM_USE_DEFAULT=1
    cpan IPC::Cmd
    export PERL_LOCAL_LIB_ROOT="$PERL_LOCAL_LIB_ROOT:/root/perl5"
    export PERL_MB_OPT="--install_base /root/perl5"
    export PERL_MM_OPT="INSTALL_BASE=/root/perl5"
    export PERL5LIB="/root/perl5/lib/perl5:$PERL5LIB"
    export PATH="/root/perl5/bin:$PATH"

    # Python3
    python3 -m venv ${HOME}/.local/pyvenv
    source ${HOME}/.local/pyvenv/bin/activate
    python3 -m pip install conan==1.62 pip==23.1.2
    PATH="${PATH}:${HOME}/.local/bin:${HOME}/.local/pyvenv"
    conan profile new --detect default
    source ${HOME}/.local/pyvenv/bin/activate
}

# PERL needs to run again.
do_perl_stuff() {
    # PERL stuff
    export PERL_MM_USE_DEFAULT=1
    cpan IPC::Cmd
    export PERL_LOCAL_LIB_ROOT="$PERL_LOCAL_LIB_ROOT:/root/perl5"
    export PERL_MB_OPT="--install_base /root/perl5"
    export PERL_MM_OPT="INSTALL_BASE=/root/perl5"
    export PERL5LIB="/root/perl5/lib/perl5:$PERL5LIB"
    export PATH="/root/perl5/bin:$PATH"
}

# Function to fetch the GitHub repository
fetch_github_repo() {
    echo "==============================="
    echo "Get the GitHub Repo"
    echo "Repo  : $HYPERONC_URL"
    echo "Branch: $HYPERONC_REV"
    echo "==============================="
    mkdir -p ${HOME}/hyperonc
    cd ${HOME}/hyperonc
    git init
    git remote add origin $HYPERONC_URL
    git fetch --depth=1 origin $HYPERONC_REV
    git reset --hard FETCH_HEAD
}

# Function to build the project
build_project() {
    echo "==============================="
    echo "Phase 3: Build the Release"
    echo "==============================="
    mkdir -p ${HOME}/hyperonc/build
    cd ${HOME}/hyperonc/build/
    cmake -DBUILD_SHARED_LIBS=OFF -DCMAKE_BUILD_TYPE=Release ..
    make
    make check
    make install
#    mkdir -p ${HOME}/hyperonc/c/build
#    cd ${HOME}/hyperonc/c/build/
#    cmake -DBUILD_SHARED_LIBS=OFF -DCMAKE_BUILD_TYPE=Release ..
#    make
}

# Main script execution
parse_args "$@"
echo "hyperonc repository URL $HYPERONC_URL"
echo "hyperonc revision $HYPERONC_REV"
install_manylinux_deps
install_rust_cbindgen
setup_perl_python_env
fetch_github_repo
do_perl_stuff
build_project

#EOF
