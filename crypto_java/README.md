This is the binding between the crypto system written in rust and the app made in java

In order to build for yourself, get https://github.com/bbqsrc/cargo-ndk

then run

`cargo ndk -t armeabi-v7a -t arm64-v8a -t x86 -o ./jniLibs build --release`

Put the whole `/jniLibs` folder into `app/src/main/` in your project folder
