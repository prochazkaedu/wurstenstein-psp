#![no_std]

const FINETUNE_TABLE: [i32; 16] = [
	65536, 65065, 64596, 64132, 63670, 63212, 62757, 62306,
	69433, 68933, 68438, 67945, 67456, 66971, 66489, 66011,
];

const SINE_TABLE: [u8; 32] = [
	0, 24, 49, 74, 97, 120, 141, 161, 180, 197, 212, 224, 235, 244, 250, 253,
	255, 253, 250, 244, 235, 224, 212, 197, 180, 161, 141, 120, 97, 74, 49, 24,
];

const ARPEGGIO_TABLE: [i32; 16] = [
	65536, 61858, 58386, 55109, 52016, 49096, 46341, 43740,
	41285, 38968, 36781, 34716, 32768, 30929, 29193, 27554,
];

const AGE_MAX: u32 = i32::MAX as u32;

fn recalculate_waveform(osc: &mut Oscillator, random: &mut u32) {
	let result = match osc.waveform {
		0 => {
			let r = SINE_TABLE[(osc.phase & 0x1F) as usize] as i32;
			if (osc.phase & 0x20) != 0 { -r } else { r }
		}
		1 => 255i32 - (((osc.phase as i32 + 0x20) & 0x3F) << 3),
		2 => 255i32 - ((osc.phase as i32 & 0x20) << 4),
		3 => {
			let v = (*random >> 20) as i32 - 255;
			*random = random.wrapping_mul(65).wrapping_add(17) & 0x1FFFFFFF;
			v
		}
		_ => 0,
	};
	osc.val = result * osc.depth as i32;
}

#[derive(Clone, Debug)]
#[derive(Default)]
pub struct PaulaChannel<'a> {
	pub sample: Option<&'a [u8]>,
	pub age: u32,
	pub currentptr: u32,
	pub length: u32,
	pub looplength: u32,
	pub period: u32,
	pub volume: i32,
	pub currentsubptr: u32,
	pub muted: bool,
}


#[derive(Clone, Debug)]
#[derive(Default)]
pub struct Sample<'a> {
	pub data: &'a [u8],
	pub actuallength: u16,
	pub looplength: u16,
}


#[derive(Clone, Debug, Default)]
pub struct Oscillator {
	pub val: i32,
	pub waveform: u8,
	pub phase: u8,
	pub speed: u8,
	pub depth: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum ModAction {
	Jump { order: i32, row: i32 },
}

#[derive(Clone, Debug)]
pub struct ModRule<'a> {
	pub order: i32,
	pub row: i32,
	pub actions: &'a [ModAction],
}

#[derive(Clone, Debug)]
#[derive(Default)]
pub struct TrackerChannel<'a> {
	pub note: u32,
	pub sample: u8,
	pub eff: u8,
	pub effval: u8,
	pub slideamount: u8,
	pub sampleoffset: u8,
	pub volume: i16,
	pub slidenote: i32,
	pub period: i32,
	pub vibrato: Oscillator,
	pub tremolo: Oscillator,
	pub samplegen: PaulaChannel<'a>,
}


#[derive(Clone, Copy, Debug)]
struct SampleHeader {
	_name: [u8; 22],
	length: u16,
	finetune: u8,
	volume: u8,
	looppoint: u16,
	_looplength: u16,
}

impl SampleHeader {
	fn from_bytes(bytes: &[u8; 30]) -> Self {
		let mut _name = [0u8; 22];
		_name.copy_from_slice(&bytes[0..22]);
		Self {
			_name,
			length: u16::from_be_bytes([bytes[22], bytes[23]]),
			finetune: bytes[24],
			volume: bytes[25],
			looppoint: u16::from_be_bytes([bytes[26], bytes[27]]),
			_looplength: u16::from_be_bytes([bytes[28], bytes[29]]),
		}
	}
}

const MAX_CHANNELS: usize = 32;

pub struct ModPlayer<'a, const N: usize = MAX_CHANNELS> {
	channels: u8,
	orders: u8,
	maxpattern: u8,
	order: u8,
	row: u8,
	tick: u8,
	maxtick: u8,
	speed: u8,
	skiporderrequest: i8,
	skiporderdestrow: u8,
	patlooprow: u8,
	patloopcycle: u8,
	samplerate: u32,
	paularate: u32,
	audiospeed: u32,
	audiotick: u32,
	random: u32,
	ch: [TrackerChannel<'a>; N],
	patterndata: &'a [u8],
	ordertable: &'a [u8],
	_sampleheaders: [SampleHeader; 31],
	samples: [Sample<'a>; 31],
	rules: Option<&'a [ModRule<'a>]>,
}

impl<'a, const N: usize> ModPlayer<'a, N> {
	pub fn new(mod_data: &'a [u8], samplerate: u32) -> Option<Self> {
		if mod_data.len() < 1084 {
			return None;
		}

		let signature = u32::from_be_bytes(mod_data[1080..1084].try_into().ok()?);

		let channels: u8 = {
			let mut c = 0u8;
			if signature == 0x4D2E4B2E || signature == 0x4D214B21 {
				c = 4;
			}
			if (signature & 0xFFFFFF00) == 0x464C5400 {
				let low = signature as u8;
				if (b'1'..=b'9').contains(&low) {
					c = low - b'0';
				}
			}
			if (signature & 0x00FFFFFF) == 0x0043484E {
				let high = (signature >> 24) as u8;
				if (b'1'..=b'9').contains(&high) {
					c = high - b'0';
				}
			}
			if (signature & 0x0000FFFF) == 0x00004348 {
				let high = ((signature >> 24) & 0xFF) as u8;
				let low = ((signature >> 16) & 0xFF) as u8;
				if high.is_ascii_digit() && low >= b'0' && high <= b'9' {
					c = (high - b'0') * 10 + (low - b'0');
				}
			}
			c
		};

		if channels < 2 || channels as usize > N.min(MAX_CHANNELS) {
			return None;
		}

		let orders = mod_data[950];
		let ordertable = &mod_data[952..952 + 128];

		let maxpattern = ordertable.iter().take(128).copied().max().unwrap_or(0) + 1;

		let pattern_data_size = 64 * 4 * channels as usize * maxpattern as usize;
		let patterndata_end = 1084 + pattern_data_size;
		if patterndata_end > mod_data.len() {
			return None;
		}
		let patterndata = &mod_data[1084..patterndata_end];

		let mut sampleheaders = [SampleHeader {
			_name: [0u8; 22],
			length: 0,
			finetune: 0,
			volume: 0,
			looppoint: 0,
			_looplength: 0,
		}; 31];

		for (i, hdr) in sampleheaders.iter_mut().enumerate() {
			let offset = 20 + i * 30;
			if offset + 30 <= mod_data.len() {
				if let Ok(bytes) = mod_data[offset..offset + 30].try_into() {
					*hdr = SampleHeader::from_bytes(bytes);
				}
			}
		}

		let mut sample_cursor = patterndata_end;
		let mut samples: [Sample<'a>; 31] = [(); 31].map(|_| Sample::default());

		for i in 0..31 {
			let hdr = sampleheaders[i];
			let length = hdr.length as usize;
			let byte_len = length * 2;

			let data = if sample_cursor + byte_len <= mod_data.len() {
				&mod_data[sample_cursor..sample_cursor + byte_len]
			} else {
				&[]
			};
			sample_cursor += byte_len;

			let looppoint = hdr.looppoint;
			let looplen = hdr._looplength;

			let (actuallength, final_looplen) = {
				let mut act = looplen;
				act = act.wrapping_add(looppoint);

				if act < 2 {
					(hdr.length, 0)
				} else if act > hdr.length {
					let lp = looppoint / 2;
					act -= lp;
					(act, act - lp)
				} else {
					(act, act - looppoint)
				}
			};

			samples[i] = Sample { data, actuallength, looplength: final_looplen };
		}

		let paularate = (3546895u64 / samplerate as u64).saturating_mul(1 << 16) as u32;

		let ch: [TrackerChannel<'a>; N] = [(); N].map(|_| {
			let mut tc = TrackerChannel::default();
			tc.samplegen.age = AGE_MAX;
			tc
		});

		Some(Self {
			channels,
			orders,
			maxpattern,
			order: 0,
			row: 0,
			tick: 0,
			maxtick: 6,
			speed: 6,
			skiporderrequest: -1,
			skiporderdestrow: 0,
			patlooprow: 0,
			patloopcycle: 0,
			samplerate,
			paularate,
			audiospeed: samplerate / 50,
			audiotick: 0,
			random: 0,
			ch,
			patterndata,
			ordertable,
			_sampleheaders: sampleheaders,
			samples,
			rules: None,
		})
	}

	pub fn set_rules(&mut self, rules: Option<&'a [ModRule<'a>]>) {
		self.rules = rules;
	}

	pub fn process(&mut self) {
		let nch = self.channels as usize;

		if self.tick == 0 {
			if let Some(rules) = self.rules {
				for rule in rules {
					if rule.order >= 0 && rule.order != self.order as i32 {
						continue;
					}
					if rule.row >= 0 && rule.row != self.row as i32 {
						continue;
					}
					for action in rule.actions {
						match action {
							ModAction::Jump { order, row } => {
								self.row = *row as u8;
								self.order = *order as u8;
							}
						}
					}
				}
			}

			self.skiporderrequest = -1;

			for i in 0..nch {
				self.ch[i].vibrato.val = 0;
				self.ch[i].tremolo.val = 0;

				let pat = self.ordertable[self.order as usize] as usize;
				let off = 4 * (i + nch * (self.row as usize + 64 * pat));
				let cell = &self.patterndata[off..off + 4];

				let note_raw = ((cell[0] as u16) << 8 | cell[1] as u16) & 0x0FFF;
				let sample_num = (cell[0] & 0xF0) | (cell[2] >> 4);
				let eff = cell[2] & 0x0F;
				let effval = cell[3];

				if self.ch[i].eff == 0 && self.ch[i].effval != 0 {
					self.ch[i].period = self.ch[i].note as i32;
				}

				if sample_num != 0 {
					let idx = if sample_num > 31 { 1 } else { sample_num } as usize - 1;
					self.ch[i].sample = sample_num;
					self.ch[i].samplegen.length = (self.samples[idx].actuallength as u32) << 1;
					self.ch[i].samplegen.looplength = (self.samples[idx].looplength as u32) << 1;
					self.ch[i].volume = self._sampleheaders[idx].volume as i16;
					self.ch[i].samplegen.sample = Some(self.samples[idx].data);
				}

				if note_raw != 0 {
					let finetune = if eff == 0x0E && (effval & 0xF0) == 0x50 {
						effval & 0x0F
					} else {
						let si = self.ch[i].sample as usize;
						if si < 31 { self._sampleheaders[si].finetune } else { 0 }
					};

					let note_fixed = (note_raw as i32).wrapping_mul(FINETUNE_TABLE[(finetune & 0x0F) as usize]) >> 16;
					self.ch[i].note = note_fixed as u32;

					if eff != 0x03 && eff != 0x05 && (eff != 0x0E || (effval & 0xF0) != 0xD0) {
						self.ch[i].samplegen.age = 0;
						self.ch[i].samplegen.currentptr = 0;
						self.ch[i].period = note_fixed;
						if self.ch[i].vibrato.waveform < 4 {
							self.ch[i].vibrato.phase = 0;
						}
						if self.ch[i].tremolo.waveform < 4 {
							self.ch[i].tremolo.phase = 0;
						}
					}
				}

				if eff != 0 || effval != 0 {
					match eff {
						0x03 => {
							if effval != 0 {
								self.ch[i].slideamount = effval;
							}
							self.ch[i].slidenote = self.ch[i].note as i32;
						}
						0x05 => {
							self.ch[i].slidenote = self.ch[i].note as i32;
						}
						0x04 => {
							if effval & 0xF0 != 0 {
								self.ch[i].vibrato.speed = effval >> 4;
							}
							if effval & 0x0F != 0 {
								self.ch[i].vibrato.depth = effval & 0x0F;
							}
							recalculate_waveform(&mut self.ch[i].vibrato, &mut self.random);
						}
						0x06 => {
							recalculate_waveform(&mut self.ch[i].vibrato, &mut self.random);
						}
						0x07 => {
							if effval & 0xF0 != 0 {
								self.ch[i].tremolo.speed = effval >> 4;
							}
							if effval & 0x0F != 0 {
								self.ch[i].tremolo.depth = effval & 0x0F;
							}
							recalculate_waveform(&mut self.ch[i].tremolo, &mut self.random);
						}
						0x0C => {
							self.ch[i].volume = if effval > 0x40 { 0x40 } else { effval as i16 };
						}
						0x09 => {
							if note_raw != 0 {
								if effval != 0 {
									self.ch[i].samplegen.currentptr = (effval as u32) << 8;
									self.ch[i].sampleoffset = effval;
								} else {
									self.ch[i].samplegen.currentptr =
										(self.ch[i].sampleoffset as u32) << 8;
								}
								self.ch[i].samplegen.age = 0;
							}
						}
						0x0B => {
							let dest = if effval >= self.orders { 0 } else { effval };
							self.skiporderrequest = dest as i8;
						}
						0x0D => {
							if self.skiporderrequest < 0 {
								self.skiporderrequest = if self.order + 1 < self.orders {
									(self.order + 1) as i8
								} else {
									0
								};
							}
							let bcd = if effval > 0x63 { 0 } else { effval };
							self.skiporderdestrow = (bcd >> 4) * 10 + (bcd & 0x0F);
						}
						0x0E => match effval >> 4 {
							0x01 => self.ch[i].period -= (effval & 0x0F) as i32,
							0x02 => self.ch[i].period += (effval & 0x0F) as i32,
							0x04 => self.ch[i].vibrato.waveform = effval & 0x07,
							0x06 => {
								if effval & 0x0F != 0 {
									if self.patloopcycle == 0 {
										self.patloopcycle = (effval & 0x0F) + 1;
									}
									if self.patloopcycle > 1 {
										self.skiporderrequest = self.order as i8;
										self.skiporderdestrow = self.patlooprow;
									}
									self.patloopcycle -= 1;
								} else {
									self.patlooprow = self.row;
								}
							}
							0x07 => self.ch[i].tremolo.waveform = effval & 0x07,
							0x0A => {
								self.ch[i].volume += (effval & 0x0F) as i16;
								if self.ch[i].volume > 0x40 {
									self.ch[i].volume = 0x40;
								}
							}
							0x0B => {
								self.ch[i].volume -= (effval & 0x0F) as i16;
								if self.ch[i].volume < 0x00 {
									self.ch[i].volume = 0x00;
								}
							}
							0x0E => {
								self.maxtick *= (effval & 0x0F) + 1 ;
							}
							_ => {}
						},
						0x0F
							if effval != 0 => {
								if effval < 0x20 {
									self.maxtick =
										(self.maxtick / self.speed) * effval;
									self.speed = effval;
								} else {
									self.audiospeed =
										self.samplerate * 125 / effval as u32 / 50;
								}
							}
						_ => {}
					}
				}

				self.ch[i].eff = eff;
				self.ch[i].effval = effval;
			}
		}

		for i in 0..nch {
			let eff = self.ch[i].eff;
			let effval = self.ch[i].effval;

			if eff != 0 || effval != 0 {
				match eff {
					0x00 => {
						match self.tick % 3 {
							0 => self.ch[i].period = self.ch[i].note as i32,
							1 => {
								let mul = ARPEGGIO_TABLE[(effval >> 4) as usize];
								self.ch[i].period =
									((self.ch[i].note as i32).wrapping_mul(mul)) >> 16;
							}
							2 => {
								let mul = ARPEGGIO_TABLE[(effval & 0x0F) as usize];
								self.ch[i].period =
									((self.ch[i].note as i32).wrapping_mul(mul)) >> 16;
							}
							_ => {}
						}
					}
					0x01 => {
						if self.tick != 0 {
							self.ch[i].period -= effval as i32;
						}
					}
					0x02 => {
						if self.tick != 0 {
							self.ch[i].period += effval as i32;
						}
					}
					0x05 => {
						if self.tick != 0 {
							if effval > 0x0F {
								self.ch[i].volume += (effval >> 4) as i16;
								if self.ch[i].volume > 0x40 {
									self.ch[i].volume = 0x40;
								}
							} else {
								self.ch[i].volume -= (effval & 0x0F) as i16;
								if self.ch[i].volume < 0x00 {
									self.ch[i].volume = 0x00;
								}
							}
						}
						if self.tick != 0 {
							self.tone_portamento(i, 0);
						}
					}
					0x03 => {
						if self.tick != 0 {
							self.tone_portamento(i, effval);
						}
					}
					0x04 => {
						if self.tick != 0 {
							self.ch[i].vibrato.phase =
								self.ch[i].vibrato.phase.wrapping_add(self.ch[i].vibrato.speed);
							recalculate_waveform(&mut self.ch[i].vibrato, &mut self.random);
						}
					}
					0x06 => {
						if self.tick != 0 {
							self.ch[i].vibrato.phase =
								self.ch[i].vibrato.phase.wrapping_add(self.ch[i].vibrato.speed);
							recalculate_waveform(&mut self.ch[i].vibrato, &mut self.random);
						}
						if self.tick != 0 {
							if effval > 0x0F {
								self.ch[i].volume += (effval >> 4) as i16;
								if self.ch[i].volume > 0x40 {
									self.ch[i].volume = 0x40;
								}
							} else {
								self.ch[i].volume -= (effval & 0x0F) as i16;
								if self.ch[i].volume < 0x00 {
									self.ch[i].volume = 0x00;
								}
							}
						}
					}
					0x0A => {
						if self.tick != 0 {
							if effval > 0x0F {
								self.ch[i].volume += (effval >> 4) as i16;
								if self.ch[i].volume > 0x40 {
									self.ch[i].volume = 0x40;
								}
							} else {
								self.ch[i].volume -= (effval & 0x0F) as i16;
								if self.ch[i].volume < 0x00 {
									self.ch[i].volume = 0x00;
								}
							}
						}
					}
					0x07 => {
						if self.tick != 0 {
							self.ch[i].tremolo.phase =
								self.ch[i].tremolo.phase.wrapping_add(self.ch[i].tremolo.speed);
							recalculate_waveform(&mut self.ch[i].tremolo, &mut self.random);
						}
					}
					0x0E => match effval >> 4 {
						0x09 => {
							if self.tick != 0 && self.tick.is_multiple_of(effval & 0x0F) {
								self.ch[i].samplegen.age = 0;
								self.ch[i].samplegen.currentptr = 0;
								self.ch[i].samplegen.currentsubptr = 0;
							}
						}
						0x0C => {
							if self.tick >= (effval & 0x0F) {
								self.ch[i].volume = 0;
							}
						}
						0x0D
							if self.tick == (effval & 0x0F) => {
								self.ch[i].samplegen.age = 0;
								self.ch[i].samplegen.currentptr = 0;
								self.ch[i].samplegen.currentsubptr = 0;
								self.ch[i].period = self.ch[i].note as i32;
							}
						_ => {}
					},
					_ => {}
				}
			}

			if self.ch[i].period < 0 {
				self.ch[i].period = 0;
			}

			if self.ch[i].period != 0 {
				let combined = self.ch[i].period + (self.ch[i].vibrato.val >> 7);
				if combined > 0 {
					self.ch[i].samplegen.period = self.paularate / combined as u32;
				} else {
					self.ch[i].samplegen.period = 0;
				}
			} else {
				self.ch[i].samplegen.period = 0;
			}

			let mut vol = self.ch[i].volume as i32 + (self.ch[i].tremolo.val >> 6);
			vol = vol.clamp(0, 64);
			self.ch[i].samplegen.volume = vol;
		}

		self.tick += 1;
		if self.tick >= self.maxtick {
			self.tick = 0;
			self.maxtick = self.speed;

			if self.skiporderrequest >= 0 {
				self.row = self.skiporderdestrow;
				self.order = self.skiporderrequest as u8;
				self.skiporderdestrow = 0;
				self.skiporderrequest = -1;
			} else {
				self.row += 1;
				if self.row >= 0x40 {
					self.row = 0;
					self.order += 1;
					if self.order >= self.orders {
						self.order = 0;
					}
				}
			}
		}
	}

	fn tone_portamento(&mut self, ch_idx: usize, effval: u8) {
		let amt = if effval == 0 {
			self.ch[ch_idx].slideamount as i32
		} else {
			effval as i32
		};
		let slidenote = self.ch[ch_idx].slidenote;
		let period = &mut self.ch[ch_idx].period;
		if slidenote > *period {
			*period += amt;
			if slidenote < *period {
				*period = slidenote;
			}
		} else if slidenote < *period {
			*period -= amt;
			if slidenote > *period {
				*period = slidenote;
			}
		}
	}

	pub fn render(&mut self, buf: &mut [i16]) {
		let samples = buf.len() / 2;
		let half_chan = (self.channels as usize / 2).max(1);
		let major_mul = 131072i32 / half_chan as i32;
		let minor_mul = 131072i32 / 3 / half_chan as i32;

		for s in 0..samples {
			if self.audiotick == 0 {
				self.process();
				self.audiotick = self.audiospeed;
			}
			self.audiotick -= 1;

			let mut l = 0i32;
			let mut r = 0i32;

			for ci in 0..self.channels as usize {
				let pch = &mut self.ch[ci].samplegen;

				if let Some(data) = pch.sample {
					if pch.looplength == 0 && pch.currentptr >= pch.length {
						continue;
					}

					while pch.currentptr >= pch.length {
						pch.currentptr = pch.currentptr.wrapping_sub(pch.looplength);
					}

					if !pch.muted {
						let mut next = pch.currentptr + 1;
						while next >= pch.length {
							if pch.looplength != 0 {
								next = next.wrapping_sub(pch.looplength);
							} else {
								next = pch.currentptr;
							}
						}

						let s1 = data[pch.currentptr as usize] as i8 as i32;
						let s2 = data[next as usize] as i8 as i32;

						let sample = (s1 * (0x10000i32 - pch.currentsubptr as i32)
							+ s2 * pch.currentsubptr as i32)
							* pch.volume
							/ 65536i32;

						match ci & 3 {
							1 | 2 => {
								l = l.wrapping_add(sample.wrapping_mul(minor_mul));
								r = r.wrapping_add(sample.wrapping_mul(major_mul));
							}
							_ => {
								l = l.wrapping_add(sample.wrapping_mul(major_mul));
								r = r.wrapping_add(sample.wrapping_mul(minor_mul));
							}
						}
					}

					pch.currentsubptr = pch.currentsubptr.wrapping_add(pch.period);
					if pch.currentsubptr >= 0x10000 {
						pch.currentptr = pch.currentptr.wrapping_add(pch.currentsubptr >> 16);
						pch.currentsubptr &= 0xFFFF;
					}

					if pch.age < AGE_MAX {
						pch.age += 1;
					}
				}
			}

			buf[s * 2] = (l / 65536) as i16;
			buf[s * 2 + 1] = (r / 65536) as i16;
		}
	}

	pub fn jump(&mut self, dest: i32) {
		let cur = self.order as i32;

		let target = match dest {
			-2 => (cur - 1).max(0) as u8,
			-1 => (cur + 1).min(self.orders as i32 - 1) as u8,
			_ => {
				let clamped = dest.max(0).min(self.orders as i32 - 1);
				clamped as u8
			}
		};

		let channels = self.channels;
		let orders = self.orders;
		let maxpattern = self.maxpattern;
		let samplerate = self.samplerate;
		let paularate = self.paularate;
		let patterndata = self.patterndata;
		let ordertable = self.ordertable;

		let _sampleheaders = self._sampleheaders;
		let samples = core::mem::take(&mut self.samples);

		*self = Self {
			channels,
			orders,
			maxpattern,
			order: 0,
			row: 0,
			tick: 0,
			maxtick: 6,
			speed: 6,
			skiporderrequest: -1,
			skiporderdestrow: 0,
			patlooprow: 0,
			patloopcycle: 0,
			samplerate,
			paularate,
			audiospeed: samplerate / 50,
			audiotick: 0,
			random: 0,
			ch: [(); N].map(|_| {
				let mut tc = TrackerChannel::default();
				tc.samplegen.age = AGE_MAX;
				tc
			}),
			patterndata,
			ordertable,
			_sampleheaders,
			samples,
			rules: None,
		};

		let mut prev_order = 0u8;
		while self.order < target {
			self.process();
			if prev_order > self.order {
				break;
			}
			prev_order = self.order;
		}
	}

	pub fn channels(&self) -> u8 {
		self.channels
	}

	pub fn order(&self) -> u8 {
		self.order
	}

	pub fn row(&self) -> u8 {
		self.row
	}

	pub fn tick(&self) -> u8 {
		self.tick
	}

	pub fn speed(&self) -> u8 {
		self.speed
	}

	pub fn track(&self) -> &[TrackerChannel<'a>] {
		&self.ch[..self.channels as usize]
	}
}
