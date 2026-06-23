# Wurstenstein 3D - PSP edition

It's Wurstenstein, the award-winning videogame, but this time for the Sony PSP, the greatest handheld game console ever created[^1].

https://github.com/user-attachments/assets/840bdd24-7227-4feb-9c5b-325ade2c21c0

[^1]: Maybe except for the Vita, which is next on the chopping block, once I get hold of one.

## Installing

Put your PSP into USB Mode, and copy the generated `./target/mipsel-sony-psp/release/EBOOT.PBP` to `/PSP/GAME/wurstenstein/EBOOT.PBP` on the PSP's storage.

## Controls

- Analog stick = movement
- L/R trigger = move camera left/right
- Square = shoot
- Cross = jump
- Start = enable flashlight
- Left/Right = change FOV
- Select = toggle profiling info

## Goal of the game

Shoot enemies and dodge bullets coming at you. Use available powerups wisely for your advantage.

You must survive for 2 minutes to win the game. The higher the score, the better you did.

## Compiling

```bash
cargo install cargo-psp # Only need to do once
git clone https://github.com/overdrivenpotato/rust-psp
git clone https://github.com/prochazkaedu/wurstenstein-psp
cd wurstenstein-psp
cargo psp --release
# If it fails, try:
#   rustup component add rust-src
# ...and try running Cargo again.
```

