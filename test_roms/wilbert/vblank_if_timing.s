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
;
; This test was written by Wilbert Pol.

; This test checks when the vblank interrupt bit (bit #0) changes and
; when the vblank interrupt is triggered.

; Verified results:
;   pass: MGB, CGB, AGS 
;   fail: 
;   not checked: DMG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

  xor a
  ldh (<IE),a
  ldh (<IF),a
  ld a,$f0     ; make sure the LY=LYC flag never gets set
  ldh (<LYC),a

test_round1:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  wait_ly 143
  xor a
  ldh (<IF),a
  nops 97
  ldh a,(<IF) ; IF = $E0 (vblank bit still reset)
  ld (round1),a

test_round2:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  wait_ly 143
  xor a
  ldh (<IF),a
  nops 98
  ldh a,(<IF) ; IF = $E1 (vblank bit gets set)
  ld (round2),a

test_round3:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  wait_ly 143
  xor a
  ldh (<IF),a
  wait_ly 14
  ldh a,(<IF) ; IF = $E1 (vblank bit stays on)
  ld (round3),a

test_round4:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  call wait_vblank_irq
  di
  nops 96
  ldh a,(<LY)  ; LY = 144
  ld (round4),a

test_round5:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  call wait_vblank_irq
  di
  nops 97
  ldh a,(<LY)  ; LY = 145
  ld (round5),a


test_finish:
  ld a,(round1)
  ld b,a
  ld a,(round2)
  ld c,a
  ld a,(round3)
  ld d,a
  ld a,(round4)
  ld e,a
  ld a,(round5)
  ld h,a
  ld a,(round6) 
  ld l,a

  save_results
  assert_b $e0
  assert_c $e1
  assert_d $e1
  assert_e 144
  assert_h 145
;  assert_l $81
  jp process_results

wait_vblank_irq:
  wait_ly 142
  ld a,$01
  ldh (<IE),a
  dec a
  ldh (<IF),a
  ei
  nops 1000
  test_failure_string "VBLANK_IRQ"

.org INTR_VEC_VBLANK
  add sp,+2
  ret

.ramsection "Test-State" slot 2
  round1 db
  round2 db
  round3 db
  round4 db
  round5 db
  round6 db
  round7 db
  round8 db
.ends

