; This file is part of Mooneye GB.
; Copyright (C) 2014-2016 Joonas Javanainen <joonas.javanainen@gmail.com>
;
; Mooneye GB is free software: you can redistribute it and/or modify
; it under the terms of the GNU General Public License as published by
; the Free Software Foundation, either version 3 of the License, or
; (at your option) any later version.
;
; Mooneye GB is distributed in the hope that it will be useful,
; but WITHOUT ANY WARRANTY; without even the implied warranty of
; MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
; GNU General Public License for more details.
;
; You should have received a copy of the GNU General Public License
; along with Mooneye GB.  If not, see <http://www.gnu.org/licenses/>.

; Tests how SCX affects the duration between STAT mode=0, STAT interrupt,
; and LY increment.
; No sprites or window.
;
; Expected behaviour:
;   (SCX mod 8) = 0   => LY increments 51 cycles after STAT interrupt
;   (SCX mod 8) = 1-4 => LY increments 50 cycles after STAT interrupt
;   (SCX mod 8) = 5-7 => LY increments 49 cycles after STAT interrupt
;   The STAT interrupt is triggered 1 clock cycle after mode=0.

; Verified results:
;   pass: MGB, CGB, AGS
;   fail: ?
;   untested: DMG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

.macro clear_interrupts
  xor a
  ldh (<IF), a
.endm

.macro scroll_x ARGS value
  ld a, value
  ldh (<SCX), a
.endm

  di
  wait_vblank
  ld a,$08
  ldh (<STAT), a
  ld a,INTR_STAT
  ldh (<IE), a

.macro perform_test_ly ARGS scanline delay_a delay_b
  ld d, scanline - 1
  ld e, scanline
  test_iter_ly scanline delay_a
  cp d
  jp nz, test_fail1
  test_iter_ly scanline delay_b
  cp e
  jp nz, test_fail2
.endm

.macro test_iter_ly ARGS scanline delay
  call setup
  call standard_ly_delay
  ; 6 + 23 + 4
  nops delay
  ; N cycles
  ld a, (hl)
.endm

.macro perform_test_m0 ARGS scanline delay_a delay_b
  ld d, scanline - 1
  test_iter_m0 scanline delay_a
  cp b
  jp nz, test_fail3
  test_iter_m0 scanline delay_b
  cp c
  jp nz, test_fail4
.endm

.macro test_iter_m0 ARGS scanline delay
  call setup
  call standard_m0_delay
  nops delay
  ld a,(hl)
  and 3
.endm

  ld hl, LY
  scroll_x $00
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $01
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $02
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $03
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $04
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $05
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $06
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $07
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5
  scroll_x $08
  perform_test_ly $42 2 3
  perform_test_ly $43 4 5

  ld hl, STAT
  ld bc, $0300
  scroll_x $00
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $01
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $02
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $03
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $04
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $05
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $06
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $07
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $08
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5

  ld hl, IF
  ld bc, $0002
  scroll_x $00
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $01
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $02
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5
  scroll_x $03
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $04
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $05
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $06
  perform_test_m0 $42 3 4
  perform_test_m0 $43 5 6
  scroll_x $07
  perform_test_m0 $42 4 5
  perform_test_m0 $43 6 7
  scroll_x $08
  perform_test_m0 $42 2 3
  perform_test_m0 $43 4 5

  test_ok

test_fail4:
  ld h,4
  jr test_fail
test_fail3:
  ld h,3
  jr test_fail
test_fail2:
  ld h,2
  jr test_fail
test_fail1:
  ld h,1
test_fail:
  ld b, a
  ldh a, (<SCX)
  save_results
  ; A = SCX
  ; B = LY value
  ; D = scanline - 1
  ; E = scanline
  test_failure_dump

standard_m0_delay:
  xor a
  ldh (<IF),a
  nops 35
  ret

standard_ly_delay:
  nops 89
  ret

setup:
  wait_vblank
  push hl
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  set 7,(hl)   ; LCD on
  pop hl
- ldh a, (<LY)
  cp d
  jr nz, -
  ret


fail_halt:
  test_failure_string "FAIL: HALT"

.org INTR_VEC_STAT
  add sp,+2
  ret
