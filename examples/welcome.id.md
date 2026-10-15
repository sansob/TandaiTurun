---
title: Selamat datang di TandaiTurun
---

# Selamat datang di TandaiTurun

TandaiTurun menampilkan Markdown seperti PDF reader menampilkan dokumen: sebagai
**halaman**, lengkap dengan zoom, thumbnail halaman dan bookmark. Kalau mau
mengubah sesuatu, tekan **Edit** (atau `Ctrl+E`): sumber Markdown terbuka dengan
pratinjau langsung di sebelahnya.

> *Tandai turun* adalah terjemahan bebas dari **mark down**. Ya, cuma itu leluconnya.

## Yang bisa kamu lakukan

- [x] Buka file `.md`, `.markdown` dan `.txt`, atau seret ke jendela
- [x] Baca dalam tampilan **bergulir**, **satu halaman** atau **dua halaman**
- [x] Zoom 25% sampai 400%, pas lebar atau pas satu halaman
- [x] Lompat lewat **Bookmark** (judul-judul) dan **Thumbnail Halaman**
- [x] Cari teks dengan `Ctrl+F`; semua hasil tampil per halaman di panel Cari
- [x] Pilih dan salin teks langsung dari halaman
- [x] Gambar diagram dari blok kode `mermaid`, tanpa browser
- [x] Susun rumus matematika: $e^{i\pi} + 1 = 0$
- [x] Edit dengan toolbar format, halaman ikut berubah selagi mengetik
- [x] Ekspor ke **PDF** (teks asli, tautan bisa diklik, bookmark) atau **HTML**
- [x] Cetak lewat penampil PDF bawaan sistem
- [x] Warna halaman kertas, sepia dan malam; huruf sans atau serif
- [x] Bahasa Inggris dan Bahasa Indonesia

## Membaca

| Aksi                    | Pintasan                  |
| ----------------------- | ------------------------- |
| Halaman berikut/sebelum | `PgDn` / `PgUp`           |
| Halaman pertama/terakhir | `Home` / `End`           |
| Perbesar / perkecil     | `Ctrl` `+` / `Ctrl` `-`   |
| Ukuran asli             | `Ctrl+1`                  |
| Pas halaman / lebar     | `Ctrl+0` / `Ctrl+2`       |
| Mode Baca               | `Ctrl+H`                  |
| Layar penuh             | `F11`                     |

Tahan `Ctrl` sambil memutar roda mouse untuk zoom. Pilih **Alat Tangan** di
toolbar untuk menggeser halaman, atau **Alat Pilih** untuk memilih teks.

## Mengedit

Editornya paham Markdown: tekan `Enter` di akhir item daftar dan butir
berikutnya (angka, atau kotak centang) langsung ditambahkan; tekan di item
kosong untuk mengakhiri daftar. `Tab` dan `Shift+Tab` mengatur indentasi.

```rust
// Blok kode diwarnai sesuai bahasanya.
fn main() {
    println!("Halo, dunia!");
}
```

Butuh halaman baru di PDF? Tulis `<!-- pagebreak -->` di barisnya sendiri.

## File-mu tetap milikmu

TandaiTurun tidak pernah mengunggah apa pun. Gambar dari internet baru dimuat
kalau kamu izinkan, jadi dokumen tidak bisa memberi tahu siapa pun bahwa kamu
membukanya; gambar yang disimpan di sebelah file-mu tampil seperti biasa.

---

TandaiTurun open source (MIT atau Apache-2.0). Kode, laporan masalah dan ide:
<https://github.com/sansob/TandaiTurun>.
