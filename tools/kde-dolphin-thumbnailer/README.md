# kde-dolphin-thumbnailer

An SVG thumbnails generator for the KDE's
[Dolphin](https://www.kde.org/applications/system/dolphin/) file manager.

## Build

```bash
# build and install C-API first in case you don't have resvg installed already
cargo build --release --manifest-path ../../crates/c-api/Cargo.toml
strip -s ../../target/release/libresvg.so # optional, but recommended
sudo cp ../../target/release/libresvg.so /usr/lib64/

# build
mkdir build
cd build
cmake .. -DCMAKE_INSTALL_PREFIX=`kf5-config --prefix` -DKDE_INSTALL_QTPLUGINDIR=`kf5-config --qt-plugins` -DCMAKE_BUILD_TYPE=Release
make

# install
sudo make install
```

## Enable

In Dolphin, go to the Settings -> Configure Dolphin -> General -> Previews.
Then disable *SVG Images* and enable *SVG Images (resvg)*.

Also, it's a good idea to reset the thumbnails cache:

```bash
rm -r ~/.cache/thumbnails
```
