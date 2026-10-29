(() => {
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const root = document.documentElement;
  root.classList.add('js');

  /* ---------- language ---------- */
  const TR = {
    'skip': 'İçeriğe geç',
    'nav.mouse': 'Fare', 'nav.eq': 'Ekolayzer', 'nav.keys': 'Tuşlar', 'nav.install': 'Kurulum',
    'hero.over': 'Terminal müzik çalar · Windows · Linux · macOS · MIT',
    'hero.h1a': 'Masaüstü müzik', 'hero.h1b': 'çalarının yaptığı her şey,', 'hero.h1c': 'seksen sütunda.',
    'hero.lede': 'Renkli albüm kapağı. Söylenen kelimeyi takip eden sözler. 10 bantlı ekolayzer. Yayın standardında ses eşitleme.',
    'hero.one': '<b>Tek bir 5,5 MB exe</b> — ffmpeg yok, codec pack yok, çalışma zamanı yok.',
    'dl.win': 'Windows için indir', 'dl.mac': 'macOS için indir', 'dl.linux': 'Linux için indir',
    'cta.other': 'Scoop, Linux, macOS',
    'hero.hint': 'Bu çalar canlı. <b>Plağa tıkla</b>, durur; <b>dalga formunu sürükle</b>, sarar; bir satıra iki kez tıkla, çalar. <kbd>p</kbd> <kbd>n</kbd> <kbd>b</kbd> <kbd>←</kbd> <kbd>→</kbd> da çalışır.',
    'demo.aria': 'Etkileşimli tlk-tune demosu, ses yok', 'demo.silent': 'demo · sessiz',
    'demo.pause': 'Duraklat', 'demo.play': 'Çal',
    'mouse.over': '01 — Kimsenin beklemediği kısım',
    'mouse.h': 'Fareyle kullanılan bir metin programı.',
    'mouse.p1': 'Plağa tıkla, durur. Dalga formunu sürükle, sarar. Satıra bir kez tıkla seçer, bir daha tıkla çalar, sağ tıkla sıraya alır. Tekerleği çevir, liste kayar.',
    'mouse.p2': 'Ayar ekranındaki her renk, her anahtar, her kaydırak da tıklanabilir. İstemedikçe hiçbir yapılandırma dosyası açman gerekmiyor. Klavye de hepsini yapar; ? ikisini birden ekrana getirir.',
    'shot.search': 'Aksanı yok sayan arama: Türkçe karakteri olmayan bir klavyede yazılan <code>dunya</code>, <code>Dünya</code>’yı buluyor.',
    'shot.search.alt': 'Türkçe karakter kullanılmadan yazılmış aramayla süzülmüş kitaplık',
    'shot.settings': 'Her renk canlı önizleme üstünde düzenleniyor. Yedi sekme, metin düzenleyici yok.',
    'shot.settings.alt': 'Ayar ekranının renkler sekmesi',
    'shot.help': 'Bütün tuşlar ve fare hareketleri, bir ? uzağında.',
    'shot.help.alt': 'Tuşları ve fare hareketlerini listeleyen yardım ekranı',
    'shot.narrow': 'Pencere daraldığında kendini topluyor; kare yukarı kaçmıyor.',
    'shot.narrow.alt': 'Aynı çalar 78 sütunda, liste sığacak şekilde kısaltılmış',
    'specs.aria': 'Rakamlarla',
    'spec.size': 'Tek exe. Kurulumun tamamı bu.',
    'spec.codecs.u': 'codec', 'spec.codecs': 'kurulmaz. MP3’ten AIFF’e, çözme Rust içinde.',
    'spec.formats.u': 'biçim',
    'spec.lufs': 'EBU R 128 eşitleme. Her parça bir kez ölçülür, bir daha asla fazla yüksek çalmaz.',
    'spec.fade.u': 'saniye çapraz geçiş', 'spec.fade': 'Varsayılan boşluksuz. Ses cihazı bir kez açılır, açık kalır.',
    'spec.os.u': 'sistem', 'spec.os': 'Windows, Linux, macOS. Etiketten CI ile derlenir, SHA256SUMS ile gelir.',
    'eq.over': '02 — Ses',
    'eq.h': 'On bant. <em>Düz eğri hesaplanmaz,</em> atlanır.',
    'eq.p': '31 Hz ile 16 kHz arası ±12 dB, çalarla gelen yedi hazır ayarla. Bir bandı sürükle ya da üstüne gelip ok tuşlarını kullan; terminalde olduğu gibi.',
    'eq.presets': 'Hazır ayarlar',
    'feat.over': '03 — Geri kalan her şey',
    'f1.h': 'Ekranda',
    'f1.a': 'Renkli albüm kapağı, dosyaya gömülü kapaktan ya da yanındaki cover.jpg’den. Windows Terminal, kitty, WezTerm, Ghostty, foot ve xterm’de gerçek piksel.',
    'f1.b': 'O an söylenen kelimesi yanan sözler; .lrc dosyasından ya da LRCLIB’den, parça başına kaydırılabilir.',
    'f1.c': 'Canlı FFT spektrumu, parçanın tamamını gösteren braille dalga formu, sese tepki veren küre.',
    'f1.d': 'Türkçe ve İngilizce, yeniden başlatmadan değişir.',
    'f2.h': 'Kitaplığın',
    'f2.a': 'Klasörler de m3u, m3u8, pls çalma listeleri de eşit derecede geçerli kök.',
    'f2.b': 'Başlıklar etiketlerden gelir, boyut ve zamana göre saklanır; büyük kitaplık bedelini bir kez öder.',
    'f2.c': 'Sıra, karıştır, tekrarla, filtreler ve albüme göre dahil beş sıralama.',
    'f2.d': 'w kurduğunu göreli yollarla m3u8 olarak yazar. l beğenir, v beğeniler, en çok çalan, son çalanlar arasında gezer.',
    'f3.h': 'Pencerenin dışında',
    'f3.a': 'Terminal arka plandayken medya tuşları çalışır.',
    'f3.b': 'Kulaklık çıkarsa cihazı yeniden açıp devam eder.',
    'f3.c': 'Uyku zamanlayıcısı ve çalanı gösteren pencere başlığı.',
    'f3.d': 'yt-dlp PATH’teyse çevrimiçi arama, akış ve indirme.',
    'keys.over': '04 — Tuşlar',
    'keys.h': 'Bu sayfa da <em>biliyor.</em>',
    'keys.p': 'Sayfanın herhangi bir yerinde p, n, b, ok tuşları, 1 ya da 2’ye bas; yukarıdaki çalar cevap verir.',
    'k.search': 'Yerelde ara', 'k.play': 'Çal / duraklat', 'k.sel': 'Seçileni çal', 'k.next': 'Sonraki', 'k.prev': 'Önceki',
    'k.back': 'Geri sar', 'k.fwd': 'İleri sar', 'k.vdown': 'Sesi kıs', 'k.vup': 'Sesi aç', 'k.shuf': 'Karıştır',
    'k.rep': 'Tekrarla', 'k.q': 'Sıraya ekle', 'k.tab': 'Panel değiştir', 'k.set': 'Ayarlar', 'k.help': 'Bütün tuşlar',
    'inst.over': '05 — Kurulum',
    'inst.h': 'Üç <em>dakika.</em>',
    'inst.tab.dl': 'İndir', 'inst.tab.src': 'Derle',
    'inst.scoop': 'Windows’ta scoop hepsini tek seferde yapar:',
    'inst.dl1': 'Sistemine uygun derlemeyi al. En çok Windows’ta yaşadı; Linux ve macOS derlemeleri daha yeni ve daha az yol katetti.',
    'inst.dl2': 'Çalıştır. Hiçbir ayar yapılmamışken Müzik ve İndirilenler klasörlerini tarar, yani genelde hemen çalacak bir şey bulunur.',
    'inst.dl3': '? bütün tuşları ve hareketleri gösterir. s ayarları açar; müziğinin gerçekten durduğu klasörü KLASORLER sekmesinden gösterirsin.',
    'inst.sums': 'indirileni doğrula',
    'inst.path': 'İsteğe bağlı. Exe bulunduğu yerden çalışır; bu adım onu PATH’e tlk-tune ve tune adlarıyla koyar ve ses dosyalarının sağ tık menüsüne “Play with tlk-tune” ekler. Yönetici hakkı gerekmez, --uninstall hepsini geri alır.',
    'inst.src': 'Rust 1.88 ya da üstü; Linux’ta ayrıca ALSA başlıkları (libasound2-dev).',
    'inst.quiet': '%APPDATA%\\tlk-tune ve %LOCALAPPDATA%\\tlk-tune dışına sen istemedikçe hiçbir şey yazılmaz.',
    'copy': 'Kopyala', 'copied': 'Kopyalandı',
    'foot.rel': 'Sürümler', 'foot.arch': 'Nasıl kurulu', 'foot.log': 'Değişiklikler', 'foot.sec': 'Güvenlik'
  };
  const EN = { 'dl.mac': 'Download for macOS', 'dl.linux': 'Download for Linux', 'demo.play': 'Play', 'copied': 'Copied' };
  let lang = 'en';
  const t = k => (lang === 'tr' ? TR[k] : EN[k]) ?? null;

  // English lives in the HTML; remember it once so switching back is lossless.
  const orig = new Map();
  const remember = (el, attr) => { if (!orig.has(el)) orig.set(el, {}); const o = orig.get(el); if (!(attr in o)) o[attr] = attr === 'html' ? el.innerHTML : attr === 'text' ? el.textContent : el.getAttribute(attr); return o[attr]; };
  function applyLang(l) {
    lang = l;
    root.lang = l;
    $$('[data-i18n]').forEach(el => { const en = remember(el, 'text'); el.textContent = l === 'tr' && TR[el.dataset.i18n] ? TR[el.dataset.i18n] : en; });
    $$('[data-i18n-html]').forEach(el => { const en = remember(el, 'html'); el.innerHTML = l === 'tr' && TR[el.dataset.i18nHtml] ? TR[el.dataset.i18nHtml] : en; });
    $$('[data-i18n-aria]').forEach(el => { const en = remember(el, 'aria-label'); el.setAttribute('aria-label', l === 'tr' && TR[el.dataset.i18nAria] ? TR[el.dataset.i18nAria] : en); });
    $$('[data-i18n-alt]').forEach(el => { const en = remember(el, 'alt'); el.alt = l === 'tr' && TR[el.dataset.i18nAlt] ? TR[el.dataset.i18nAlt] : en; });
    setDownload();
    syncPlay();
    $$('.num').forEach(n => { if (n.dataset.done) n.textContent = fmtNum(+n.dataset.to, +(n.dataset.dec || 0)); });
    try { localStorage.setItem('tlk-tune-lang', l); } catch {}
  }
  $('.lang').addEventListener('click', () => applyLang(lang === 'en' ? 'tr' : 'en'));

  /* ---------- download button follows the visitor's system ---------- */
  const plat = ((navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || navigator.userAgent).toLowerCase();
  const os = /mac|iphone|ipad/.test(plat) ? 'mac' : /linux|x11/.test(plat) && !/android/.test(plat) ? 'linux' : 'win';
  const ASSET = { win: 'tlk-tune-windows-x86_64.exe', mac: 'tlk-tune-macos-aarch64', linux: 'tlk-tune-linux-x86_64' };
  function setDownload() {
    $('#dl').href = 'https://github.com/Talkdedsec/tlk-tune/releases/latest/download/' + ASSET[os];
    const lbl = $('#dl-label');
    lbl.textContent = os === 'win' ? (lang === 'tr' ? TR['dl.win'] : 'Download for Windows') : t('dl.' + os);
  }

  /* ---------- 80-column ruler ---------- */
  const ticks = $('.ruler-ticks');
  for (let c = 1; c <= 80; c++) {
    const x = ((c - .5) / 80 * 100) + '%';
    if (c % 10 === 0) { const b = document.createElement('b'); b.textContent = c / 10; b.style.left = x; ticks.append(b); }
    else { const i = document.createElement('i'); if (c % 5 === 0) i.className = 'five'; i.style.left = x; ticks.append(i); }
  }
  const cur = $('.ruler-cur'), read = $('.ruler-read');
  addEventListener('pointermove', e => {
    if (e.pointerType !== 'mouse') return;
    const col = Math.min(80, Math.max(1, Math.floor(e.clientX / innerWidth * 80) + 1));
    cur.style.transform = `translateX(${(col - 1) * 100}%)`;
    read.textContent = 'col ' + String(col).padStart(2, '0');
    document.body.classList.add('ptr');
  }, { passive: true });
  addEventListener('scroll', () => document.body.classList.toggle('scrolled', scrollY > 8), { passive: true });

  /* ---------- the player ---------- */
  const TRACKS = [
    { name: 'Neon Terminal', dur: 214, fmt: 'FLAC', size: '4.0 MB', seed: 3.1 },
    { name: 'Vinyl Static', dur: 243, fmt: 'MP3', size: '9.3 MB', seed: 7.4 },
    { name: 'Night Shift', dur: 229, fmt: 'OPUS', size: '5.1 MB', seed: 11.7 },
    { name: 'Braille Rain', dur: 198, fmt: 'FLAC', size: '3.6 MB', seed: 5.9 }
  ];
  const LYRICS = [
    'terminal hums, the fan keeps time', 'raw mode on, the cursor gone', 'eighty columns wide, nothing else',
    'and the record turns', 'braille rain on a black screen', 'count the frames, forty a second',
    'the disc keeps turning', 'no window, no mouse, no chrome', 'just the sound and the grid'
  ];
  const LINE = 5.2;
  const st = { i: 0, sel: 0, t: 43, playing: !reduce, vol: 72, angle: 0 };
  const waveEl = $('.wave'), rec = $('.record'), cvs = $('.record canvas'), ctx = cvs.getContext('2d');
  const fmt = s => String(Math.floor(s / 60)).padStart(2, '0') + ':' + String(Math.floor(s % 60)).padStart(2, '0');

  // record: 48×48 pixel art, label mark rotates, sheen stays put like real vinyl under a lamp
  const img = ctx.createImageData(48, 48);
  const C = { vinyl: [27, 22, 19], groove: [35, 29, 24], sheen1: [45, 37, 31], sheen2: [64, 52, 43], edge: [50, 41, 35], label: [255, 138, 31], ring: [196, 86, 15], mark: [120, 50, 8], hole: [15, 12, 10] };
  function drawRecord() {
    const d = img.data;
    for (let y = 0; y < 48; y++) for (let x = 0; x < 48; x++) {
      const dx = x - 23.5, dy = y - 23.5, r = Math.hypot(dx, dy), k = (y * 48 + x) * 4;
      let c = null;
      if (r <= 23.4) {
        const a = Math.atan2(dy, dx);
        if (r < 1.8) c = C.hole;
        else if (r < 8.6) c = Math.cos(a - st.angle) > .93 && r > 3 ? C.mark : C.label;
        else if (r < 9.6) c = C.ring;
        else if (r > 22.4) c = C.edge;
        else {
          const s = Math.cos(2 * (a - .75));
          c = s > .9 ? C.sheen2 : s > .62 ? C.sheen1 : (Math.floor(r * 1.25) % 2 ? C.groove : C.vinyl);
          if (s > .62 && Math.floor(r * 1.25) % 2 === 0) c = C.sheen1;
        }
      }
      if (c) { d[k] = c[0]; d[k + 1] = c[1]; d[k + 2] = c[2]; d[k + 3] = 255; } else d[k + 3] = 0;
    }
    ctx.putImageData(img, 0, 0);
  }

  // braille waveform: 3 text rows, 12 dots tall, mirrored around the middle
  let chars = 0, rows = [], lastP = -1;
  const BITS = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
  function amp(i, n, seed) {
    const x = i / n, env = Math.min(1, x * 9, (1 - x) * 7);
    const v = .22 + .42 * Math.abs(Math.sin(i * .045 + seed)) * (.55 + .45 * Math.sin(i * .011 * seed + 1.3))
      + .2 * Math.abs(Math.sin(i * 1.7 + seed * 3) * Math.sin(i * .37));
    return Math.max(.05, Math.min(1, v * env));
  }
  function buildWave() {
    // measure a real row of braille, since fallback fonts give these glyphs their own width
    waveEl.innerHTML = '<div><span>' + '⣿'.repeat(100) + '</span></div>';
    const cw = waveEl.firstChild.firstChild.getBoundingClientRect().width / 100;
    chars = Math.max(10, Math.floor(waveEl.clientWidth / cw));
    const seed = TRACKS[st.i].seed, n = chars * 2;
    rows = [[], [], []];
    for (let c = 0; c < chars; c++) {
      const h = [amp(c * 2, n, seed), amp(c * 2 + 1, n, seed)].map(a => Math.round(a * 6));
      for (let ro = 0; ro < 3; ro++) {
        let bits = 0;
        for (let k = 0; k < 2; k++) for (let yy = 0; yy < 4; yy++) {
          const y = ro * 4 + yy;
          if (y >= 6 - h[k] && y < 6 + h[k]) bits |= BITS[k][yy];
        }
        rows[ro].push(String.fromCharCode(0x2800 + bits));
      }
    }
    waveEl.innerHTML = rows.map(() => '<div><span class="p"></span><span></span></div>').join('');
    lastP = -1;
    paintWave();
  }
  function paintWave() {
    const p = Math.round(st.t / TRACKS[st.i].dur * chars);
    if (p === lastP) return;
    lastP = p;
    [...waveEl.children].forEach((div, ro) => {
      div.firstChild.textContent = rows[ro].slice(0, p).join('');
      div.lastChild.textContent = rows[ro].slice(p).join('');
    });
  }

  const lyr = $('.lyrics');
  lyr.innerHTML = LYRICS.map(l => '<li>' + l.split(' ').map(w => `<span class="w">${w}</span>`).join(' ') + '</li>').join('');
  const lis = [...lyr.children];
  function paintLyrics() {
    const lt = st.t % (LYRICS.length * LINE), li = Math.floor(lt / LINE), wi = (lt - li * LINE) / LINE;
    lis.forEach((el, j) => {
      el.classList.toggle('cur', j === li);
      if (j === li) { const ws = el.children, on = Math.floor(wi * ws.length * 1.15); [...ws].forEach((w, q) => w.classList.toggle('on', q <= on)); }
    });
  }

  const rowsEl = $('.list .rows');
  function paintList() {
    rowsEl.innerHTML = TRACKS.map((tr, j) =>
      `<li class="${j === st.sel ? 'sel' : ''} ${j === st.i ? 'playing' : ''}" data-j="${j}"><span>${j + 1}</span><span>${tr.name}</span><span>Talkdedsec</span><span>${fmt(tr.dur)}</span></li>`).join('');
  }
  rowsEl.addEventListener('click', e => {
    const li = e.target.closest('li'); if (!li) return;
    const j = +li.dataset.j;
    if (st.sel === j) { load(j); st.playing = true; syncPlay(); } else { st.sel = j; paintList(); }
  });

  function load(j) {
    st.i = (j + TRACKS.length) % TRACKS.length; st.sel = st.i; st.t = 0;
    const tr = TRACKS[st.i];
    $('#m-name').textContent = tr.name; $('#m-fmt').textContent = tr.fmt; $('#m-size').textContent = tr.size;
    $('#t-dur').textContent = fmt(tr.dur);
    waveEl.setAttribute('aria-valuemax', tr.dur);
    paintList(); buildWave(); tick(0, true);
  }
  function syncPlay() {
    $('#b-play').textContent = st.playing ? 'PAUSE' : 'PLAY';
    rec.setAttribute('aria-pressed', st.playing);
    rec.setAttribute('aria-label', st.playing ? (t('demo.pause') || 'Pause') : t('demo.play'));
  }
  function flash(id) { const b = $(id); b.classList.add('flash'); setTimeout(() => b.classList.remove('flash'), 160); }
  const toggle = () => { st.playing = !st.playing; syncPlay(); flash('#b-play'); };
  const seek = s => { st.t = Math.max(0, Math.min(TRACKS[st.i].dur - .01, s)); tick(0, true); };
  const setVol = v => {
    st.vol = Math.max(0, Math.min(100, Math.round(v)));
    const on = Math.round(st.vol / 5);
    $('.vol-on').textContent = '#'.repeat(on); $('.vol-off').textContent = '-'.repeat(20 - on);
    $('.vol-n').textContent = st.vol + '%'; $('.vol').setAttribute('aria-valuenow', st.vol);
  };

  rec.addEventListener('click', toggle);
  $('#b-play').addEventListener('click', toggle);
  $('#b-next').addEventListener('click', () => { load(st.i + 1); flash('#b-next'); });
  $('#b-prev').addEventListener('click', () => { st.t > 3 ? seek(0) : load(st.i - 1); flash('#b-prev'); });

  function drag(el, onX) {
    el.addEventListener('pointerdown', e => {
      el.setPointerCapture(e.pointerId);
      const go = ev => { const r = el.getBoundingClientRect(); onX(Math.max(0, Math.min(1, (ev.clientX - r.left) / r.width))); };
      go(e);
      const up = () => { el.removeEventListener('pointermove', go); el.removeEventListener('pointerup', up); el.removeEventListener('pointercancel', up); };
      el.addEventListener('pointermove', go); el.addEventListener('pointerup', up); el.addEventListener('pointercancel', up);
    });
  }
  drag(waveEl, f => seek(f * TRACKS[st.i].dur));
  const volEl = $('.vol');
  volEl.addEventListener('pointerdown', e => {
    // only the bar between the brackets maps to volume
    volEl.setPointerCapture(e.pointerId);
    const r0 = volEl.querySelector('.vol-on').getBoundingClientRect(), r1 = volEl.querySelector('.vol-off').getBoundingClientRect();
    const left = r0.width ? r0.left : r1.left, w = r0.width + r1.width;
    const set = ev => setVol((ev.clientX - left) / w * 100);
    set(e);
    const up = () => { volEl.removeEventListener('pointermove', set); volEl.removeEventListener('pointerup', up); volEl.removeEventListener('pointercancel', up); };
    volEl.addEventListener('pointermove', set); volEl.addEventListener('pointerup', up); volEl.addEventListener('pointercancel', up);
  });
  waveEl.addEventListener('keydown', e => {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') { e.preventDefault(); e.stopPropagation(); seek(st.t + (e.key === 'ArrowLeft' ? -5 : 5)); }
  });
  volEl.addEventListener('keydown', e => {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') { e.preventDefault(); e.stopPropagation(); setVol(st.vol + (e.key === 'ArrowLeft' ? -5 : 5)); }
  });

  let last = performance.now(), visible = true;
  new IntersectionObserver(([e]) => { visible = e.isIntersecting; }).observe($('.term'));
  function tick(dt, force) {
    if (st.playing && dt) {
      st.t += dt;
      st.angle += dt * 3.2;
      if (st.t >= TRACKS[st.i].dur) load(st.i + 1);
    }
    if (force || (st.playing && visible)) {
      drawRecord(); paintWave(); paintLyrics();
      $('#t-now').textContent = fmt(st.t);
      waveEl.setAttribute('aria-valuenow', Math.floor(st.t)); waveEl.setAttribute('aria-valuetext', fmt(st.t));
    }
  }
  function frame(now) { const dt = Math.min(.1, (now - last) / 1000); last = now; tick(dt); requestAnimationFrame(frame); }

  /* ---------- page-wide keys, the same ones tlk-tune uses ---------- */
  const caps = new Map($$('.caps li').map(li => [li.dataset.key, li]));
  addEventListener('keydown', e => {
    const tg = e.target instanceof Element ? e.target : document.body;
    if (e.ctrlKey || e.metaKey || e.altKey || tg.closest('input, textarea, [contenteditable], .band, [role="tab"]')) return;
    const k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
    const li = caps.get(k);
    if (li) { li.classList.add('hit'); setTimeout(() => li.classList.remove('hit'), 220); }
    if (k === 'p') toggle();
    else if (k === 'n') { load(st.i + 1); flash('#b-next'); }
    else if (k === 'b') { load(st.i - 1); flash('#b-prev'); }
    else if (k === 'ArrowLeft' || k === 'ArrowRight') { if (tg.closest('button, a')) return; seek(st.t + (k === 'ArrowLeft' ? -5 : 5)); }
    else if (k === '1') setVol(st.vol - 5);
    else if (k === '2') setVol(st.vol + 5);
    else if (k === '?') $('#keys').scrollIntoView({ behavior: reduce ? 'auto' : 'smooth' });
  });

  /* ---------- equaliser ---------- */
  const HZ = ['31', '62', '125', '250', '500', '1k', '2k', '4k', '8k', '16k'];
  const PRESETS = {
    flat: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    bass: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0],
    treble: [0, 0, 0, 0, 0, 1, 2.5, 4, 5, 5],
    vocal: [-2, -2, -1, 1, 3, 4, 3, 1, 0, -1],
    rock: [4, 3, 1, -1, -2, 0, 2, 3, 4, 4],
    loudness: [5, 4, 2, 0, -1, -1, 0, 2, 4, 5],
    night: [-4, -3, -1, 1, 2, 2, 1, 0, -2, -3]
  };
  const eq = { v: PRESETS.rock.slice(), anim: 0 };
  const bandsEl = $('.bands'), presetsEl = $('.presets'), curve = $('.eq-curve path');
  bandsEl.innerHTML = HZ.map((h, i) => `<div class="band" role="slider" tabindex="0" aria-label="${h}Hz" aria-valuemin="-12" aria-valuemax="12" data-i="${i}"><span class="db"></span><div class="cells">${'<i></i>'.repeat(25)}</div><span class="hz">${h}</span></div>`).join('');
  presetsEl.innerHTML = Object.keys(PRESETS).map(p => `<button type="button" data-p="${p}" aria-pressed="false">${p}</button>`).join('');
  const bands = $$('.band');
  function paintEq() {
    bands.forEach((b, i) => {
      const v = eq.v[i], cells = b.querySelector('.cells').children, top = Math.round(12 - v);
      for (let r = 0; r < 25; r++) {
        cells[r].className = (r === 12 ? 'zero ' : '') + ((v >= 0 ? r >= top && r <= 12 : r >= 12 && r <= top) && Math.round(v) !== 0 ? 'on' : '');
      }
      const s = (v > 0 ? '+' : v < 0 ? '−' : '') + Math.abs(v).toFixed(Number.isInteger(v) ? 0 : 1);
      b.querySelector('.db').textContent = s;
      b.setAttribute('aria-valuenow', v); b.setAttribute('aria-valuetext', s + ' dB');
    });
    const pts = eq.v.map((v, i) => [(i + .5) / 10 * 1000, (12 - v) / 24 * 250]);
    let d = `M0 ${pts[0][1]} L${pts[0][0]} ${pts[0][1]}`;
    for (let i = 1; i < pts.length; i++) { const [x0, y0] = pts[i - 1], [x1, y1] = pts[i], mx = (x0 + x1) / 2; d += ` C${mx} ${y0} ${mx} ${y1} ${x1} ${y1}`; }
    curve.setAttribute('d', d + ` L1000 ${pts[9][1]}`);
    const c0 = bands[0].querySelector('.cells'), svg = curve.parentNode;
    svg.style.top = c0.offsetTop + 'px'; svg.style.height = c0.offsetHeight + 'px';
    const match = Object.keys(PRESETS).find(p => PRESETS[p].every((x, i) => x === eq.v[i]));
    $$('button', presetsEl).forEach(b => b.setAttribute('aria-pressed', b.dataset.p === match));
  }
  function toPreset(p) {
    const from = eq.v.slice(), to = PRESETS[p], t0 = performance.now(), D = reduce ? 0 : 420, my = ++eq.anim;
    const step = now => {
      if (my !== eq.anim) return;
      const k = D ? Math.min(1, (now - t0) / D) : 1, e = 1 - Math.pow(1 - k, 4);
      eq.v = from.map((f, i) => k === 1 ? to[i] : Math.round((f + (to[i] - f) * e) * 2) / 2);
      paintEq();
      if (k < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }
  presetsEl.addEventListener('click', e => { const b = e.target.closest('button'); if (b) toPreset(b.dataset.p); });
  bands.forEach((b, i) => {
    const cells = b.querySelector('.cells');
    b.addEventListener('pointerdown', e => {
      eq.anim++;
      b.setPointerCapture(e.pointerId);
      const set = ev => { const r = cells.getBoundingClientRect(); eq.v[i] = Math.max(-12, Math.min(12, Math.round(12 - (ev.clientY - r.top) / r.height * 24))); paintEq(); };
      set(e);
      const up = () => { b.removeEventListener('pointermove', set); b.removeEventListener('pointerup', up); b.removeEventListener('pointercancel', up); };
      b.addEventListener('pointermove', set); b.addEventListener('pointerup', up); b.addEventListener('pointercancel', up);
    });
    b.addEventListener('keydown', e => {
      const d = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1, PageUp: 3, PageDown: -3 }[e.key];
      if (d === undefined && e.key !== 'Home' && e.key !== 'End') return;
      e.preventDefault(); eq.anim++;
      eq.v[i] = e.key === 'Home' ? 12 : e.key === 'End' ? -12 : Math.max(-12, Math.min(12, Math.round(eq.v[i]) + d));
      paintEq();
    });
  });
  paintEq();

  /* ---------- install tabs + copy ---------- */
  const tabs = $$('[role="tab"]');
  function pick(tab) {
    tabs.forEach(x => { const on = x === tab; x.setAttribute('aria-selected', on); x.tabIndex = on ? 0 : -1; $('#' + x.getAttribute('aria-controls')).hidden = !on; });
  }
  tabs.forEach((tab, i) => {
    tab.addEventListener('click', () => pick(tab));
    tab.addEventListener('keydown', e => {
      const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
      if (!d) return;
      e.preventDefault(); const n = tabs[(i + d + tabs.length) % tabs.length]; pick(n); n.focus();
    });
  });
  if (os !== 'win') pick($('#tab-dl'));
  $$('.copy').forEach(b => b.addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(b.previousElementSibling.textContent); } catch { return; }
    b.textContent = t('copied'); b.classList.add('ok');
    setTimeout(() => { b.textContent = lang === 'tr' ? TR.copy : 'Copy'; b.classList.remove('ok'); }, 1400);
  }));

  /* ---------- numbers + reveals ---------- */
  const fmtNum = (v, dec) => (v < 0 ? '−' : '') + Math.abs(v).toLocaleString(lang === 'tr' ? 'tr-TR' : 'en-US', { minimumFractionDigits: dec, maximumFractionDigits: dec });
  function count(n) {
    const to = +n.dataset.to, dec = +(n.dataset.dec || 0), t0 = performance.now(), D = 1100;
    n.dataset.done = 1;
    if (reduce || to === 0) { n.textContent = fmtNum(to, dec); return; }
    const step = now => { const k = Math.min(1, (now - t0) / D), e = 1 - Math.pow(1 - k, 4); n.textContent = fmtNum(to * e, dec); if (k < 1) requestAnimationFrame(step); };
    requestAnimationFrame(step);
  }
  const io = new IntersectionObserver(es => es.forEach(e => {
    if (!e.isIntersecting) return;
    e.target.classList.add('in');
    const n = e.target.querySelector('.num'); if (n) count(n);
    io.unobserve(e.target);
  }), { rootMargin: '0px 0px -12% 0px' });
  $$('.reveal').forEach(el => io.observe(el));

  /* ---------- start ---------- */
  let saved = null;
  try { saved = localStorage.getItem('tlk-tune-lang'); } catch {}
  const q = new URLSearchParams(location.search).get('lang');
  applyLang(q === 'tr' || q === 'en' ? q : saved || (/^tr\b/i.test(navigator.language) ? 'tr' : 'en'));
  setVol(st.vol);
  load(0); st.t = 43; tick(0, true);
  let rw;
  addEventListener('resize', () => { clearTimeout(rw); rw = setTimeout(() => { buildWave(); paintEq(); }, 120); });
  document.fonts.ready.then(buildWave);
  requestAnimationFrame(() => { root.classList.add('ready'); requestAnimationFrame(frame); });
})();
