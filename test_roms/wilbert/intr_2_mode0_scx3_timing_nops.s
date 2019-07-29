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

; Tests how long does it take to get from STAT=mode2 interrupt to mode0 when SCX=3
; without using the HALT instruction.
; No sprites, scroll, or window

; Verified results:
;   pass: MGB, CGB, AGS
;   fail: -

.incdir "../../common"
.include "common.s"

.macro clear_interrupts
  xor a
  ldh (<IF), a
.endm

.macro wait_mode ARGS mode
- ldh a, (<STAT)
  and $03
  cp mode
  jr nz, -
.endm

  di
  wait_vblank

.macro test_iter ARGS delay
  call setup_and_wait_mode2
  nops delay
  ld a, (hl)
  and $03
.endm

  ld hl, STAT
  ld a,$03
  ldh (<SCX),a
  ld a, INTR_STAT
  ldh (<IE), a
  test_iter 49
  ld d, a
  test_iter 50
  ld e, a
  call setup_and_wait_mode2_before_int
  ld b, c
  call setup_and_wait_mode2_after_int
  save_results
  assert_b $02
  assert_c $01
  assert_d $03
  assert_e $00
  jp process_results

setup_and_wait_mode2:
  wait_ly $42
  wait_mode $00
  wait_mode $03
  ld a, %00100000
  ldh (<STAT), a
  clear_interrupts
  ld c, 0
  ei

  nops 2000
  jp fail_halt

.macro test_mode2_irq ARGS delay
  wait_ly $42
  wait_mode $00
  wait_mode $03
  ld a, %00100000
  ldh (<STAT), a
  clear_interrupts
  ld c, 0
  ei
  nop
  inc c
  nops delay
  inc c
  nops 1000
  jp fail_halt
.endm

setup_and_wait_mode2_before_int:
  test_mode2_irq 66

setup_and_wait_mode2_after_int:
  test_mode2_irq 67

fail_halt:
  test_failure_string "FAIL: HALT"

.org INTR_VEC_STAT
  add sp,+2
  ret

