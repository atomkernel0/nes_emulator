# NES Emulator in Rust 🦀🎮

A cycle-accurate Nintendo Entertainment System (NES) emulator built with Rust and SDL2. This project aims to create a fully functional emulator capable of playing classic NES games, with a strong focus on clean code, accurate emulation, and detailed documentation.

## 🚀 Features

### ✅ CPU (Ricoh 2A03 - 6502 core)

- **Full 6502 Instruction Set**: Implements all official opcodes.
- **Undocumented Opcodes**: Includes support for most common illegal/undocumented opcodes used in many games.
- **Accurate Addressing Modes**: All 13 addressing modes are precisely implemented.
- **Cycle-Accurate Emulation**: Handles additional cycles for page-crossing branches and memory accesses.
- **Interrupts**: Correctly handles NMI (Non-Maskable Interrupts) from the PPU and IRQ (Interrupt Requests) from the APU and mappers.

### ✅ PPU (Picture Processing Unit)

- **Frame Rendering**: Renders a full 256x240 frame.
- **Sprite and Background Rendering**: Supports up to 64 sprites per frame, with priority handling (behind/in front of background).
- **Sprite 0 Hit Detection**: Correctly detects collisions between sprite 0 and the background, a crucial timing mechanism for many games.
- **VRAM and OAM**: Full emulation of Video RAM, Object Attribute Memory (sprite data), and palette memory.
- **Scrolling**: Manages Name Table, Attribute Table, and fine/coarse scroll registers.
- **Mirroring**: Supports Horizontal, Vertical, and Four-Screen mirroring via the iNES mapper implementation.

### ✅ APU (Audio Processing Unit)

- **5-Channel Audio Synthesis**:
  - 2 Pulse Wave channels (square waves).
  - 1 Triangle Wave channel.
  - 1 Noise channel.
  - 1 Delta Modulation Channel (DMC) for playing DPCM samples.
- **Envelopes and Sweeps**: Volume envelopes and frequency sweeps are implemented for the pulse channels.
- **Length Counters**: All channels support length counters for note duration.
- **Audio Sampling**: Generates and outputs audio samples, which are played back via SDL2.

### ✅ Cartridge & Mapper

- **iNES Format**: Loads games from the standard `.nes` file format.
- **Mapper 0 (NROM)**: Fully supported, allowing a large number of early NES titles to be played.

### ✅ System Bus

- **Memory Mapping**: Correctly maps all system components (RAM, PPU, APU, Cartridge) into the CPU's address space.
- **DMA Transfers**: Emulates OAM DMA for fast sprite memory transfers.

### ✅ Input

- **Joypad Support**: Full support for one controller.
- **Keyboard Mapping**: Maps keyboard keys to the NES joypad for gameplay.

## 🛠️ Build & Run

### 1. Install Dependencies

This project uses **SDL2** for windowing, rendering, and audio. You need to install it on your system.

**macOS (using Homebrew):**

```bash
brew install sdl2
```

**Ubuntu/Debian:**

```bash
sudo apt-get install libsdl2-dev
```

**Windows:**

1.  Download the **SDL2 development libraries** for MinGW from the [SDL2 release page](https://github.com/libsdl-org/SDL/releases) (e.g., `SDL2-devel-2.xx.x-mingw.tar.gz`).
2.  Unzip the archive.
3.  Copy the contents of the `x86_64-w64-mingw32` directory into your MinGW-w64 toolchain directory. You may need to set the `SDL2_PATH` environment variable to the SDL2 folder.

### 2. Compile & Run

Once dependencies are installed, you can build and run the emulator using Cargo.

```bash
# Clone the repository
git clone https://github.com/atomkernel0/nes_emulator.git
cd nes_emulator

# Build the project
cargo build --release

# Run the emulator with a game ROM
# (You must provide your own legally-owned ROM file)
cargo run --release -- path/to/your/game.nes
```

## ⌨️ Controls

The keyboard is mapped to the NES controller as follows:

| NES Button | Keyboard Key       |
| :--------- | :----------------- |
| **D-Pad**  | Arrow Keys         |
| **A**      | `A`                |
| **B**      | `S`                |
| **Start**  | `Enter` / `Return` |
| **Select** | `Space`            |

- **`ESC`**: Quit the emulator.
- **`R`**: Reset the emulator.

## 🏛️ Architecture

```
src/
├── main.rs          # Entry point, main game loop, SDL2 initialization
├── cpu.rs           # 6502 CPU emulation and instruction implementation
├── bus.rs           # System bus, connects all components (CPU, PPU, APU)
├── apu.rs           # Audio Processing Unit (all 5 sound channels)
├── ppu/             # Picture Processing Unit
│   ├── mod.rs       # Main PPU logic, registers, and timing
│   └── registers/   # PPU register-specific logic (control, mask, etc.)
├── render/          # Rendering helpers
│   ├── frame.rs     # Represents a single rendered frame
│   └── palette.rs   # NES color palette
├── cartridge.rs     # Cartridge loading and mapper implementation
├── joypad.rs        # Controller input handling
└── opcodes.rs       # 6502 opcode definitions and lookup table
```

## 📋 TODO & Future Work

While the emulator is quite capable, there are still many features to add for broader compatibility and a better user experience.

- [ ] **PPU Upgrade**: PPU is not well implemented.
- [ ] **More Mappers**: Implement common mappers like MMC1, MMC3, UxROM to support more games.
- [ ] **Save States**: Implement functionality to save and load the emulator's state.
- [ ] **Debugger**: Create a debugging interface to inspect CPU registers, memory, and PPU state.
- [ ] **UI Improvements**: Add a simple GUI for loading ROMs and configuring settings.
- [ ] **Performance Optimizations**: Profile and optimize the code for better performance.

## 📚 Resources

- [**NESdev Wiki**](https://wiki.nesdev.com/): The ultimate resource for NES development and emulation.
- [**6502 Instruction Set**](http://www.6502.org/tutorials/6502opcodes.html): A comprehensive guide to the 6502 CPU.
- [**One Lone Coder's NES Emulator Series**](https://www.youtube.com/playlist?list=PLLwK93hM93Z13I_oK2y2i3sO5l2Ktzg4i): A fantastic video series that served as an inspiration.
- [**Writing NES Emulator in Rust**](https://bugzmanov.github.io/nes_ebook/): A fantastic tutorial that served as the base for the actual codebase.

## 📄 Licence

This project is licensed under the GPLv3. See the `LICENSE` file for more details.

---

**\*Disclaimer**: This project is for educational purposes only. You are responsible for obtaining game ROMs legally.\*
