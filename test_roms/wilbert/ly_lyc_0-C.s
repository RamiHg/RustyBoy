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

; Check when the LY==LYC flags get (re)set and when interrupts are
; triggered around line 0.

; Verified results:
;   pass: CGB, AGS
;   fail: MGB
;   not checked: DMG, SGB, SGB2, AGB

.incdir "../../common"
.include "common.s"

  di
  xor a
  ldh (<IE),a
  ldh (<IF),a
  ld a,0
  ldh (<LYC),a
  ld a,%01000000
  ldh (<STAT),a

test_round1:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 108
  ldh a,(<LY) ; LY = 153
  ld (round1),a

test_round2:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 109
  ldh a,(<LY) ; LY = 0
  ld (round2),a

test_round3:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 109
  ldh a,(<STAT) ; STAT = $C1
  ld (round3),a

test_round4:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 110
  ldh a,(<STAT) ; STAT = $C5
  ld (round4),a

test_round5:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 109
  ldh a,(<IF) ; IF = $E1
  ld (round5),a

test_round6:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 110
  ldh a,(<IF) ; IF = $E3
  ld (round6),a

test_round7:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 220
  ldh a,(<STAT) ; STAT = $C5
  ld (round7),a

test_round8:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 221
  ldh a,(<STAT) ; STAT = $C4
  ld (round8),a

test_round9:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 222
  ldh a,(<STAT) ; STAT = $C6
  ld (round9),a

test_round10:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 334
  ldh a,(<STAT) ; STAT = $C4
  ld (round10),a

test_round11:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 335
  ldh a,(<STAT) ; STAT = $C0
  ld (round11),a

test_round12:
  wait_ly 144
  ld hl,LCDC
  res 7,(hl)   ; LCD off
  nop
  set 7,(hl)   ; LCD on
  xor a
  ldh (<IF),a
  wait_ly 152
  nops 336
  ldh a,(<STAT) ; STAT = $C2
  ld (round12),a


test_finish:
  ld a,(round1)
  ld b,a
  ld a,(round2)
  rla
  rla
  rla
  rla
  ld c,a
  push bc

  ld a,(round3)
  rla
  rla
  rla
  rla
  and $F0
  ld b,a
  ld a,(round4)
  and $0F
  or b
  ld b,a

  ld a,(round5)
  rla
  rla
  rla
  rla
  and $F0
  ld c,a
  ld a,(round6)
  and $0F
  or c
  ld c,a

  ld a,(round7)
  rla
  rla
  rla
  rla
  and $F0
  ld d,a
  ld a,(round8) 
  and $0F
  or d
  ld d,a

  ld a,(round9)
  ld e,a

  ld a,(round10)
  rla
  rla
  rla
  rla
  and $F0
  ld h,a
  ld a,(round11)
  and $0F
  or h
  ld h,a

  ld a,(round12)
  ld l,a
  pop af

  save_results
  assert_a 153
  assert_f $00
  assert_b $15
  assert_c $13
  assert_d $55
  assert_e $C6
  assert_h $44
  assert_l $C2
  jp process_results

.ramsection "Test-State" slot 2
  round1 db
  round2 db
  round3 db
  round4 db
  round5 db
  round6 db
  round7 db
  round8 db
  round9 db
  round10 db
  round11 db
  round12 db
.ends
