use crate::cartridge::Mirroring;
use registers::addr::AddrRegister;
use registers::control::ControlRegister;
use registers::mask::MaskRegister;
use registers::scroll::ScrollRegister;
use registers::status::StatusRegister;

pub mod registers;

pub struct NesPPU {
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub ctrl: ControlRegister,
    pub mask: MaskRegister,
    pub status: StatusRegister,
    pub scroll: ScrollRegister,
    pub addr: AddrRegister,
    pub vram: [u8; 2048],

    pub oam_addr: u8,
    pub oam_data: [u8; 256],
    pub palette_table: [u8; 32],

    internal_data_buf: u8,

    pub scanline: u16,
    cycles: usize,
    pub nmi_interrupt: Option<u8>,
    
    // Compteur de frames pour le debugging et les statistiques
    pub frame_count: u64,
    
    // Support pour les techniques avancées
    pub fine_x_scroll: u8,
    pub temp_vram_addr: u16,
    pub write_toggle: bool,
    
    // Historique des changements pour le split scroll et autres effets
    pub scroll_changes: Vec<(u16, u8, u8)>, // (scanline, x, y)
    pub palette_changes: Vec<(u16, usize, u8, u8)>, // (scanline, cycle, addr, value)
    pub ctrl_changes: Vec<(u16, usize, u8)>, // (scanline, cycle, value)
}

pub trait PPU {
    fn write_to_ctrl(&mut self, value: u8);
    fn write_to_mask(&mut self, value: u8);
    fn read_status(&mut self) -> u8;
    fn write_to_oam_addr(&mut self, value: u8);
    fn write_to_oam_data(&mut self, value: u8);
    fn read_oam_data(&self) -> u8;
    fn write_to_scroll(&mut self, value: u8);
    fn write_to_ppu_addr(&mut self, value: u8);
    fn write_to_data(&mut self, value: u8);
    fn read_data(&mut self) -> u8;
    fn write_oam_dma(&mut self, value: &[u8; 256]);
}

impl NesPPU {
    pub fn new_empty_rom() -> Self {
        NesPPU::new(vec![0; 2048], Mirroring::Horizontal)
    }

    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        NesPPU {
            chr_rom: chr_rom,
            mirroring: mirroring,
            ctrl: ControlRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            oam_addr: 0,
            scroll: ScrollRegister::new(),
            addr: AddrRegister::new(),
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            palette_table: [0; 32],
            internal_data_buf: 0,

            cycles: 0,
            scanline: 0,
            nmi_interrupt: None,
            frame_count: 0,
            
            // Initialisation des nouvelles fonctionnalités
            fine_x_scroll: 0,
            temp_vram_addr: 0,
            write_toggle: false,
            scroll_changes: Vec::new(),
            palette_changes: Vec::new(),
            ctrl_changes: Vec::new(),
        }
    }

    // Horizontal:
    //   [ A ] [ a ]
    //   [ B ] [ b ]

    // Vertical:
    //   [ A ] [ B ]
    //   [ a ] [ b ]
    /// Calcule l'adresse miroir pour la VRAM selon le type de mirroring
    ///
    /// Le NES utilise différents types de mirroring pour économiser la mémoire:
    /// - Horizontal: les nametables A et a sont identiques, B et b sont identiques
    /// - Vertical: les nametables A et B sont identiques, a et b sont identiques
    pub fn mirror_vram_addr(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; // mirror down 0x3000-0x3eff to 0x2000 - 0x2eff
        let vram_index = mirrored_vram - 0x2000; // to vram vector
        let name_table = vram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    /// Incrémente l'adresse VRAM selon le bit de contrôle
    /// - Si le bit 2 du registre de contrôle est 0: incrémente de 1 (mode horizontal)
    /// - Si le bit 2 du registre de contrôle est 1: incrémente de 32 (mode vertical)
    fn increment_vram_addr(&mut self) {
        self.addr.increment(self.ctrl.vram_addr_increment());
    }

    /// Avance le PPU d'un nombre donné de cycles avec support pour les effets avancés
    /// Retourne true si un frame complet a été rendu
    ///
    /// Le PPU NES fonctionne avec des cycles précis:
    /// - 341 cycles par scanline
    /// - 262 scanlines par frame (NTSC)
    /// - Support pour split scroll, changements de palette mid-frame, etc.
    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut remaining_cycles = cycles as usize;
        
        while remaining_cycles > 0 {
            // Calculer combien de cycles on peut traiter dans cette scanline
            let cycles_until_next_scanline = 341 - self.cycles;
            let cycles_to_process = remaining_cycles.min(cycles_until_next_scanline);
            
            // Traiter cycle par cycle pour les effets mid-scanline
            for _ in 0..cycles_to_process {
                self.cycles += 1;
                
                // Appliquer les changements de palette programmés pour ce cycle
                self.apply_palette_changes_for_cycle();
                
                // Appliquer les changements de contrôle programmés pour ce cycle
                self.apply_ctrl_changes_for_cycle();
                
                // Vérifier le sprite 0 hit pendant la scanline visible
                if self.scanline < 240 && self.is_sprite_0_hit(self.cycles) {
                    self.status.set_sprite_zero_hit(true);
                }
                
                // Gestion des scanlines spéciales
                if self.cycles == 341 {
                    self.end_of_scanline();
                    if self.scanline >= 262 {
                        return self.end_of_frame();
                    }
                    break;
                }
            }
            
            remaining_cycles -= cycles_to_process;
        }
        
        false
    }
    
    /// Gère la fin d'une scanline
    fn end_of_scanline(&mut self) {
        self.cycles = 0;
        self.scanline += 1;
        
        // Appliquer les changements de scroll programmés pour cette scanline
        self.apply_scroll_changes_for_scanline();
        
        // Scanline 241: début du VBlank
        if self.scanline == 241 {
            self.status.set_vblank_status(true);
            self.status.set_sprite_zero_hit(false);
            if self.ctrl.generate_vblank_nmi() {
                self.nmi_interrupt = Some(1);
            }
        }
        
        // Scanline 261: pré-render, reset des flags
        if self.scanline == 261 {
            self.status.set_sprite_zero_hit(false);
            self.status.reset_vblank_status();
        }
    }
    
    /// Gère la fin d'un frame
    fn end_of_frame(&mut self) -> bool {
        self.scanline = 0;
        self.nmi_interrupt = None;
        self.status.set_sprite_zero_hit(false);
        self.status.reset_vblank_status();
        self.frame_count = self.frame_count.wrapping_add(1);
        
        // Nettoyer les historiques des changements du frame précédent
        self.scroll_changes.clear();
        self.palette_changes.clear();
        self.ctrl_changes.clear();
        
        true
    }
    
    /// Applique les changements de scroll programmés pour la scanline actuelle
    fn apply_scroll_changes_for_scanline(&mut self) {
        for &(target_scanline, x, y) in &self.scroll_changes {
            if target_scanline == self.scanline {
                // Appliquer le changement de scroll
                self.scroll.write(x);
                self.scroll.write(y);
            }
        }
    }
    
    /// Applique les changements de palette programmés pour le cycle actuel
    fn apply_palette_changes_for_cycle(&mut self) {
        let current_cycle = self.cycles;
        for &(target_scanline, target_cycle, addr, value) in &self.palette_changes {
            if target_scanline == self.scanline && target_cycle == current_cycle {
                if addr < 32 {
                    self.palette_table[addr as usize] = value;
                }
            }
        }
    }
    
    /// Applique les changements de contrôle programmés pour le cycle actuel
    fn apply_ctrl_changes_for_cycle(&mut self) {
        let current_cycle = self.cycles;
        for &(target_scanline, target_cycle, value) in &self.ctrl_changes {
            if target_scanline == self.scanline && target_cycle == current_cycle {
                self.ctrl.update(value);
            }
        }
    }

    /// Récupère et efface l'interruption NMI en attente
    /// Retourne Some(1) si une NMI était en attente, None sinon
    pub fn poll_nmi_interrupt(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }
    
    /// Retourne le nombre de frames rendues depuis l'initialisation
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }
    
    /// Remet à zéro le compteur de frames
    pub fn reset_frame_count(&mut self) {
        self.frame_count = 0;
    }
    
    /// Programme un changement de scroll pour une scanline donnée (split scroll)
    pub fn schedule_scroll_change(&mut self, scanline: u16, x: u8, y: u8) {
        self.scroll_changes.push((scanline, x, y));
    }
    
    /// Programme un changement de palette pour un cycle donné
    pub fn schedule_palette_change(&mut self, scanline: u16, cycle: usize, addr: usize, value: u8) {
        self.palette_changes.push((scanline, cycle, addr.try_into().unwrap(), value));
    }
    
    /// Programme un changement de registre de contrôle pour un cycle donné
    pub fn schedule_ctrl_change(&mut self, scanline: u16, cycle: usize, value: u8) {
        self.ctrl_changes.push((scanline, cycle, value));
    }
    
    /// Efface tous les changements programmés
    pub fn clear_scheduled_changes(&mut self) {
        self.scroll_changes.clear();
        self.palette_changes.clear();
        self.ctrl_changes.clear();
    }
    
    /// Retourne des informations de debug sur l'état du PPU
    pub fn debug_info(&self) -> String {
        format!(
            "PPU Debug Info:\n\
             - Scanline: {}\n\
             - Cycle: {}\n\
             - Frame: {}\n\
             - VBlank: {}\n\
             - Sprite 0 Hit: {}\n\
             - Scroll X: {}, Y: {}\n\
             - Changements programmés: {} scroll, {} palette, {} ctrl",
            self.scanline,
            self.cycles,
            self.frame_count,
            self.status.is_in_vblank(),
            self.status.is_sprite_zero_hit(),
            self.scroll.scroll_x,
            self.scroll.scroll_y,
            self.scroll_changes.len(),
            self.palette_changes.len(),
            self.ctrl_changes.len()
        )
    }

    /// Détecte si le sprite 0 entre en collision avec l'arrière-plan
    /// Ceci est crucial pour le timing précis dans les jeux NES
    fn is_sprite_0_hit(&self, cycle: usize) -> bool {
        let y = self.oam_data[0] as usize;
        let x = self.oam_data[3] as usize;
        
        // Le sprite 0 hit se produit quand:
        // 1. On est sur la même scanline que le sprite 0
        // 2. On a atteint ou dépassé la position X du sprite 0
        // 3. Les sprites sont activés dans le registre mask
        // 4. L'arrière-plan est également activé
        (y == self.scanline as usize)
            && x <= cycle
            && self.mask.show_sprites()
            && self.mask.show_background()
    }
}

impl PPU for NesPPU {
    fn write_to_ctrl(&mut self, value: u8) {
        let before_nmi_status = self.ctrl.generate_vblank_nmi();
        self.ctrl.update(value);
        if !before_nmi_status && self.ctrl.generate_vblank_nmi() && self.status.is_in_vblank() {
            self.nmi_interrupt = Some(1);
        }
    }

    fn write_to_mask(&mut self, value: u8) {
        self.mask.update(value);
    }

    fn read_status(&mut self) -> u8 {
        let data = self.status.snapshot();
        self.status.reset_vblank_status();
        self.addr.reset_latch();
        self.scroll.reset_latch();
        data
    }

    fn write_to_oam_addr(&mut self, value: u8) {
        self.oam_addr = value;
    }

    fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_addr as usize] = value;
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_addr as usize]
    }

    fn write_to_scroll(&mut self, value: u8) {
        self.scroll.write(value);
    }

    fn write_to_ppu_addr(&mut self, value: u8) {
        self.addr.update(value);
    }

    fn write_to_data(&mut self, value: u8) {
        let addr = self.addr.get();
        match addr {
            0..=0x1fff => println!("attempt to write to chr rom space {}", addr),
            0x2000..=0x2fff => {
                self.vram[self.mirror_vram_addr(addr) as usize] = value;
            }
            0x3000..=0x3eff => unimplemented!("addr {} shouldn't be used in reality", addr),

            //Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let add_mirror = addr - 0x10;
                self.palette_table[(add_mirror - 0x3f00) as usize] = value;
            }
            0x3f00..=0x3fff => {
                self.palette_table[(addr - 0x3f00) as usize] = value;
            }
            _ => panic!("unexpected access to mirrored space {}", addr),
        }
        self.increment_vram_addr();
    }

    fn read_data(&mut self) -> u8 {
        let addr = self.addr.get();

        self.increment_vram_addr();

        match addr {
            // CHR ROM - utilise le buffer interne pour la lecture différée
            0..=0x1fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.chr_rom[addr as usize];
                result
            }
            
            // VRAM nametables - utilise le buffer interne pour la lecture différée
            0x2000..=0x2fff => {
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(addr) as usize];
                result
            }
            
            // Espace miroir de 0x2000-0x2fff
            0x3000..=0x3eff => {
                let mirrored_addr = addr - 0x1000;
                let result = self.internal_data_buf;
                self.internal_data_buf = self.vram[self.mirror_vram_addr(mirrored_addr) as usize];
                result
            }

            // Palette RAM avec mirroring - lecture immédiate (pas de buffer)
            // Les adresses $3F10/$3F14/$3F18/$3F1C sont des miroirs de $3F00/$3F04/$3F08/$3F0C
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                let mirrored_addr = addr - 0x10;
                self.palette_table[(mirrored_addr - 0x3f00) as usize]
            }

            // Palette RAM normale - lecture immédiate
            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            
            _ => panic!("Accès inattendu à l'espace mémoire miroir à l'adresse 0x{:04X}", addr),
        }
    }

    fn write_oam_dma(&mut self, data: &[u8; 256]) {
        // DMA (Direct Memory Access) pour transférer 256 octets vers l'OAM
        // Ceci prend normalement 513 ou 514 cycles CPU selon l'alignement
        for x in data.iter() {
            self.oam_data[self.oam_addr as usize] = *x;
            self.oam_addr = self.oam_addr.wrapping_add(1);
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_ppu_vram_writes() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);
        ppu.write_to_data(0x66);

        assert_eq!(ppu.vram[0x0305], 0x66);
    }

    #[test]
    fn test_ppu_vram_reads() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.addr.get(), 0x2306);
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_reads_cross_page() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x0200] = 0x77;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
    }

    #[test]
    fn test_ppu_vram_reads_step_32() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0b100);
        ppu.vram[0x01ff] = 0x66;
        ppu.vram[0x01ff + 32] = 0x77;
        ppu.vram[0x01ff + 64] = 0x88;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0xff);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        assert_eq!(ppu.read_data(), 0x77);
        assert_eq!(ppu.read_data(), 0x88);
    }

    // Horizontal: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 a ]
    //   [0x2800 B ] [0x2C00 b ]
    #[test]
    fn test_vram_horizontal_mirror() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to a

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to B

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from b
    }

    // Vertical: https://wiki.nesdev.com/w/index.php/Mirroring
    //   [0x2000 A ] [0x2400 B ]
    //   [0x2800 a ] [0x2C00 b ]
    #[test]
    fn test_vram_vertical_mirror() {
        let mut ppu = NesPPU::new(vec![0; 2048], Mirroring::Vertical);

        ppu.write_to_ppu_addr(0x20);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x66); //write to A

        ppu.write_to_ppu_addr(0x2C);
        ppu.write_to_ppu_addr(0x05);

        ppu.write_to_data(0x77); //write to b

        ppu.write_to_ppu_addr(0x28);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x66); //read from a

        ppu.write_to_ppu_addr(0x24);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into buffer
        assert_eq!(ppu.read_data(), 0x77); //read from B
    }

    #[test]
    fn test_read_status_resets_latch() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x21);
        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_ne!(ppu.read_data(), 0x66);

        ppu.read_status();

        ppu.write_to_ppu_addr(0x23);
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load_into_buffer
        assert_eq!(ppu.read_data(), 0x66);
    }

    #[test]
    fn test_ppu_vram_mirroring() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_ctrl(0);
        ppu.vram[0x0305] = 0x66;

        ppu.write_to_ppu_addr(0x63); //0x6305 -> 0x2305
        ppu.write_to_ppu_addr(0x05);

        ppu.read_data(); //load into_buffer
        assert_eq!(ppu.read_data(), 0x66);
        // assert_eq!(ppu.addr.read(), 0x0306)
    }

    #[test]
    fn test_read_status_resets_vblank() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.status.set_vblank_status(true);

        let status = ppu.read_status();

        assert_eq!(status >> 7, 1);
        assert_eq!(ppu.status.snapshot() >> 7, 0);
    }

    #[test]
    fn test_oam_read_write() {
        let mut ppu = NesPPU::new_empty_rom();
        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_data(0x66);
        ppu.write_to_oam_data(0x77);

        ppu.write_to_oam_addr(0x10);
        assert_eq!(ppu.read_oam_data(), 0x66);

        ppu.write_to_oam_addr(0x11);
        assert_eq!(ppu.read_oam_data(), 0x77);
    }

    #[test]
    fn test_oam_dma() {
        let mut ppu = NesPPU::new_empty_rom();

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        ppu.write_to_oam_addr(0x10);
        ppu.write_oam_dma(&data);

        ppu.write_to_oam_addr(0xf); //wrap around
        assert_eq!(ppu.read_oam_data(), 0x88);

        ppu.write_to_oam_addr(0x10);
        ppu.write_to_oam_addr(0x77);
        ppu.write_to_oam_addr(0x11);
        ppu.write_to_oam_addr(0x66);
    }
}
