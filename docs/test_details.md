# Integration Test Details

### MooneyeGB

**Passes:**

<br /> div-timing
<br /> boot-regs/-dmgABC
<br /> add-sp-e-timing
<br /> call-timing
<br /> call-cc-timing
<br /> call-cc-timing2
<br /> call-timing2
<br /> di-timing
<br /> ei-sequence
<br /> ei-timing
<br /> halt-ime0-ei
<br /> halt-ime1-timing
<br /> if-ie-registers
<br /> halt-ime0-nointr-timing
<br /> intr-timing
<br /> halt-ime1-timing2
<br /> jp-cc-timing
<br /> oam-dma/reg-read
<br /> oam-dma/basic
<br /> jp-timing
<br /> ld-hl-sp-e-timing
<br /> oam-dma-restart
<br /> pop-timing
<br /> oam-dma-start
<br /> oam-dma-timing
<br /> ppu/intr-2-0-timing
<br /> ppu/intr-1-2-timing
<br /> ppu/intr-2-mode0-timing
<br /> ppu/intr-2-mode3-timing
<br /> ppu/intr-2-oam-ok-timing
<br /> rapid-di-ei
<br /> push-timing
<br /> ret-cc-timing
<br /> ppu/vblank-stat-intr
<br /> reti-intr-timing
<br /> ret-timing
<br /> timer/rapid-toggle
<br /> reti-timing
<br /> rst-timing
<br /> timer/tim00
<br /> timer/tim00-div-trigger
<br /> timer/tim01
<br /> timer/tim01-div-trigger
<br /> timer/tim10
<br /> ppu/hblank-ly-scx-timing
<br /> timer/tim10-div-trigger
<br /> timer/tim11
<br /> timer/tim11-div-trigger
<br /> timer/tima-reload
<br /> timer/tima-write-reloading
<br /> timer/tma-write-reloading
<br /> timer/div-write

**Fails:**

interrupts/ie_push. I just simply haven't had the time or interest to fix this one. I'm sure it
wouldn't be too difficult.

### Wilbert Pol

**Passes:**

<br /> intr_2_0_timing 
<br /> intr_1_2_timing 
<br /> intr_2_mode0_scx1_timing_nops 
<br /> intr_0_timing 
<br /> intr_1_timing 
<br /> intr_2_mode0_scx2_timing_nops 
<br /> intr_2_mode0_scx3_timing_nops 
<br /> intr_2_mode0_scx4_timing_nops 
<br /> intr_2_mode0_scx5_timing_nops 
<br /> intr_2_mode0_scx6_timing_nops 
<br /> intr_2_mode0_timing 
<br /> intr_2_mode0_scx7_timing_nops 
<br /> intr_2_mode3_timing 
<br /> intr_2_mode0_scx8_timing_nops 
<br /> intr_2_timing 
<br /> hblank_ly_scx_timing_nops 
<br /> hblank_ly_scx_timing 
<br /> lcdon_mode_timing 
<br /> ly00_01_mode0_2 
<br /> ly00_mode0_2 
<br /> ly00_mode1_0 
<br /> ly00_mode2_3 
<br /> ly143_144_145 
<br /> ly00_mode3_0 
<br /> ly143_144_152_153 
<br /> ly143_144_mode0_1 
<br /> ly143_144_mode3_0 
<br /> ly_lyc_144 
<br /> ly_lyc 
<br /> vblank_if_timing 
<br /> ly_lyc_153 
<br /> vblank_stat_intr 
<br /> ly_lyc_0 
<br /> ly_new_frame 
<br /> hblank_ly_scx_timing_variant_nops 

### Blargh

All CPU instruction tests pass. All instruction time tests pass.
