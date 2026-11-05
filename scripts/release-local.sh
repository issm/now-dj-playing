#!/bin/bash
set +x

RELEASE_BASEDIR=$1
if [ -z "$RELEASE_BASEDIR" ]; then
    echo "Error: RELEASE_BASEDIR を第1引数で指定してください" >&2
    exit 1
fi

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APPS_BASEDIR=$PROJECT_ROOT/apps

#
# server
#
echo
echo "----------------------------------------------------------------"
echo "server"
echo "----------------------------------------------------------------"
set -x
RELEASE_DIR=$RELEASE_BASEDIR/server
cd $APPS_BASEDIR/server
cargo build --release
cp -v \
    $APPS_BASEDIR/server/target/release/ndp-server \
    $RELEASE_DIR/
set +x

#
# viewer
#
echo
echo "----------------------------------------------------------------"
echo "viewer"
echo "----------------------------------------------------------------"
RELEASE_DIR=$RELEASE_BASEDIR/viewer
set -x
cd $APPS_BASEDIR/viewer
yarn tauri build
rm -rf $RELEASE_DIR/now-dj-playing.app
cp -rv \
    $APPS_BASEDIR/viewer/src-tauri/target/release/bundle/macos/now-dj-playing.app \
    $RELEASE_DIR/
codesign --force --deep --sign - $RELEASE_DIR/now-dj-playing.app
cp -v $PROJECT_ROOT/scripts/setup-viewer.mac.sh $RELEASE_DIR/
set +x

#
# publisher
#
echo
echo "----------------------------------------------------------------"
echo "publisher"
echo "----------------------------------------------------------------"
RELEASE_DIR=$RELEASE_BASEDIR/publisher
set -x
cd $APPS_BASEDIR/publisher
cargo build --release
cp -v \
    $APPS_BASEDIR/publisher/target/release/ndp-publish \
    $RELEASE_DIR/
set +x

#
# publisher-gui
#
echo
echo "----------------------------------------------------------------"
echo "publisher-gui"
echo "----------------------------------------------------------------"
RELEASE_DIR=$RELEASE_BASEDIR/publisher-gui
set -x
cd $APPS_BASEDIR/publisher-gui
yarn tauri build
rm -rf $RELEASE_DIR/ndp-publish-gui.app
cp -rv \
    $APPS_BASEDIR/publisher-gui/src-tauri/target/release/bundle/macos/ndp-publish-gui.app \
    $RELEASE_DIR/
codesign --force --deep --sign - $RELEASE_DIR/ndp-publish-gui.app
cp -v $PROJECT_ROOT/scripts/setup-publisher-gui.mac.sh $RELEASE_DIR/
set +x

#
# publisher-gui (darwin)
#   Intel Mac (x86_64-apple-darwin) 向け
#   事前に `rustup target add x86_64-apple-darwin` が必要
#
echo
echo "----------------------------------------------------------------"
echo "publisher-gui (darwin)"
echo "----------------------------------------------------------------"
TARGET=x86_64-apple-darwin
RELEASE_DIR=$RELEASE_BASEDIR/publisher-gui-darwin
set -x
cd $APPS_BASEDIR/publisher-gui
yarn tauri build --target $TARGET
mkdir -p $RELEASE_DIR
rm -rf $RELEASE_DIR/ndp-publish-gui.app
cp -rv \
    $APPS_BASEDIR/publisher-gui/src-tauri/target/$TARGET/release/bundle/macos/ndp-publish-gui.app \
    $RELEASE_DIR/
codesign --force --deep --sign - $RELEASE_DIR/ndp-publish-gui.app
cp -v $PROJECT_ROOT/scripts/setup-publisher-gui.mac.sh $RELEASE_DIR/
set +x

exit 0
