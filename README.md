# NES Emulator 🎮

Un émulateur Nintendo Entertainment System (NES) écrit en Rust, avec une émulation précise du processeur 6502.

## 🚀 Fonctionnalités

### CPU 6502 ✅
- **Émulation complète du processeur 6502** avec tous les modes d'adressage
- **Gestion précise des cycles d'horloge** avec cycles supplémentaires lors des franchissements de page
- **Instructions arithmétiques** : ADC, SBC avec calcul correct de l'overflow
- **Instructions logiques** : AND, EOR, ORA
- **Instructions de transfert** : LDA, LDX, LDY, STA, STX, STY
- **Instructions de pile** : PHA, PLA, PHP, PLP, JSR, RTS, RTI
- **Instructions de branchement** : BEQ, BNE, BCS, BCC, BMI, BPL, BVS, BVC
- **Instructions de comparaison** : CMP, CPX, CPY
- **Instructions de décalage** : ASL, LSR, ROL, ROR
- **Instructions d'incrémentation/décrémentation** : INC, DEC, INX, INY, DEX, DEY
- **Gestion des flags** : Carry, Zero, Interrupt Disable, Decimal Mode, Break, Overflow, Negative

### Cartridge & ROM ✅
- **Support du format iNES** (.nes)
- **Gestion des mappers** (mapper 0 supporté)
- **Mirroring** : Horizontal, Vertical, Four-Screen
- **Validation des ROMs** avec détection des formats non supportés

### Bus & Mémoire ✅
- **Mapping mémoire correct** :
  - RAM : `0x0000-0x1FFF` (avec mirroring)
  - PPU Registers : `0x2000-0x3FFF` (préparé)
  - Cartridge ROM : `0x8000-0xFFFF`
- **Gestion des accès mémoire** avec protection en écriture de la ROM

## 🛠️ Architecture

```
src/
├── main.rs          # Point d'entrée
├── cpu.rs           # Émulation du processeur 6502
├── bus.rs           # Bus système et mapping mémoire
├── cartridge.rs     # Gestion des cartouches NES
└── opcodes.rs       # Définitions des opcodes
```

## 🧪 Tests

Le projet inclut une suite de tests complète pour valider l'émulation :

```bash
cargo test
```

Tests disponibles :
- ✅ Instructions de base (LDA, TAX, INX)
- ✅ Opérations arithmétiques
- ✅ Gestion des débordements
- ✅ Lecture depuis la mémoire
- ✅ Validation des cartouches

## 🚀 Utilisation

```bash
# Cloner le projet
git clone git@github.com:atomkernel0/nes_emulator.git
cd nes_emulator

# Compiler et tester
cargo build
cargo test

# Exécuter (en développement)
cargo run
```

## 📋 TODO

### Prochaines étapes
- [ ] **PPU (Picture Processing Unit)** - Rendu graphique
- [ ] **APU (Audio Processing Unit)** - Son et musique
- [ ] **Contrôleurs** - Input des joueurs
- [ ] **Mappers supplémentaires** (1, 2, 3, etc.)
- [ ] **Interface utilisateur** - Fenêtre de jeu
- [ ] **Sauvegarde d'état** - Save states
- [ ] **Debugger** - Outils de débogage

### Améliorations CPU
- [ ] **Instructions illégales** du 6502
- [ ] **Timing précis** cycle par cycle
- [ ] **Interruptions** (NMI, IRQ)

## 🎯 Objectifs

L'objectif est de créer un émulateur NES complet et précis, capable de faire tourner les jeux classiques comme :
- Super Mario Bros.
- The Legend of Zelda
- Metroid
- Mega Man
- Et bien d'autres !

## 🔧 Détails techniques

### Précision de l'émulation
- **Cycles d'horloge** : Gestion des cycles supplémentaires lors des franchissements de page
- **Flags du processeur** : Calcul précis de tous les flags (N, V, B, D, I, Z, C)
- **Stack** : Émulation correcte de la pile à `0x0100-0x01FF`
- **Vecteurs d'interruption** : Support des vecteurs de reset, NMI et IRQ

### Architecture 6502
Le processeur 6502 est un CPU 8-bit avec :
- **3 registres** : A (Accumulator), X, Y (Index)
- **1 registre de statut** : P (Processor Status)
- **1 compteur de programme** : PC (Program Counter)
- **1 pointeur de pile** : SP (Stack Pointer)

## 📚 Ressources

- [6502 Instruction Set](http://www.6502.org/tutorials/6502opcodes.html)
- [NESdev Wiki](https://wiki.nesdev.com/)
- [6502 Overflow Flag Explained](http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html)

## 📄 Licence

Ce projet est sous licence GPL v3.0. Voir le fichier `LICENSE` pour plus de détails.

---

**Note** : Ce projet est à des fins éducatives et de préservation du patrimoine vidéoludique. Assurez-vous de posséder légalement les ROMs que vous utilisez.