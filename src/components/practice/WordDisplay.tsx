import type { PracticeWord } from "../../lib/types";

const KANA_TO_ROMAJI: { [key: string]: string } = {
  // Hiragana
  'あ': 'a', 'い': 'i', 'う': 'u', 'え': 'e', 'お': 'o',
  'か': 'ka', 'き': 'ki', 'く': 'ku', 'け': 'ke', 'こ': 'ko',
  'が': 'ga', 'ぎ': 'gi', 'ぐ': 'gu', 'げ': 'ge', 'ご': 'go',
  'さ': 'sa', 'し': 'shi', 'す': 'su', 'せ': 'se', 'そ': 'so',
  'ざ': 'za', 'じ': 'ji', 'ず': 'zu', 'ぜ': 'ze', 'ぞ': 'zo',
  'た': 'ta', 'ち': 'chi', 'つ': 'tsu', 'て': 'te', 'と': 'to',
  'だ': 'da', 'ぢ': 'ji', 'づ': 'zu', 'で': 'de', 'ど': 'do',
  'な': 'na', 'に': 'ni', 'ぬ': 'nu', 'ね': 'ne', 'の': 'no',
  'は': 'ha', 'ひ': 'hi', 'ふ': 'fu', 'へ': 'he', 'ほ': 'ho',
  'ba': 'ba', 'び': 'bi', 'ぶ': 'bu', 'べ': 'be', 'ぼ': 'bo', // wait: 'ば' is ba
  'ば': 'ba',
  'ぱ': 'pa', 'ぴ': 'pi', 'ぷ': 'pu', 'ぺ': 'pe', 'ぽ': 'po',
  'ま': 'ma', 'み': 'mi', 'む': 'mu', 'め': 'me', 'も': 'mo',
  'や': 'ya', 'ゆ': 'yu', 'よ': 'yo',
  'ら': 'ra', 'り': 'ri', 'る': 'ru', 'れ': 're', 'ろ': 'ro',
  'わ': 'wa', 'を': 'wo', 'ん': 'n',
  
  // Katakana
  'ア': 'a', 'イ': 'i', 'ウ': 'u', 'エ': 'e', 'オ': 'o',
  'カ': 'ka', 'キ': 'ki', 'ク': 'ku', 'ケ': 'ke', 'コ': 'ko',
  'ガ': 'ga', 'ギ': 'gi', 'グ': 'gu', 'ゲ': 'ge', 'ゴ': 'go',
  'サ': 'sa', 'シ': 'shi', 'ス': 'su', 'セ': 'se', 'ソ': 'so',
  'ザ': 'za', 'ジ': 'ji', 'ズ': 'zu', 'ゼ': 'ze', 'ゾ': 'zo',
  'タ': 'ta', 'チ': 'chi', 'ツ': 'tsu', 'テ': 'te', 'ト': 'to',
  'ダ': 'da', 'ヂ': 'ji', 'ヅ': 'zu', 'デ': 'de', 'ド': 'do',
  'ナ': 'na', 'ニ': 'ni', 'ヌ': 'nu', 'ネ': 'ne', 'ノ': 'no',
  'ハ': 'ha', 'ヒ': 'hi', 'フ': 'fu', 'ヘ': 'he', 'ホ': 'ho',
  'バ': 'ba', 'ビ': 'bi', 'ブ': 'bu', 'ベ': 'be', 'ボ': 'bo',
  'パ': 'pa', 'ピ': 'pi', 'プ': 'pu', 'ペ': 'pe', 'ポ': 'po',
  'マ': 'ma', 'ミ': 'mi', 'ム': 'mu', 'メ': 'me', 'モ': 'mo',
  'ヤ': 'ya', 'ユ': 'yu', 'ヨ': 'yo',
  'ラ': 'ra', 'リ': 'ri', 'ル': 'ru', 'レ': 're', 'ロ': 'ro',
  'ワ': 'wa', 'ヲ': 'wo', 'ン': 'n'
};

const KANA_COMBINATIONS: { [key: string]: string } = {
  // Hiragana Yōon
  'きゃ': 'kya', 'きゅ': 'kyu', 'きょ': 'kyo',
  'ぎゃ': 'gya', 'ぎゅ': 'gyu', 'ぎょ': 'gyo',
  'しゃ': 'sha', 'しゅ': 'shu', 'しょ': 'sho',
  'じゃ': 'ja', 'じゅ': 'ju', 'じょ': 'jo',
  'ちゃ': 'cha', 'ちゅ': 'chu', 'ちょ': 'cho',
  'にゃ': 'nya', 'にゅ': 'nyu', 'にょ': 'nyo',
  'ひゃ': 'hya', 'ひゅ': 'hyu', 'ひょ': 'hyo',
  'びゃ': 'bya', 'びゅ': 'byu', 'びょ': 'byo',
  'ぴゃ': 'pya', 'ぴゅ': 'pyu', 'ぴょ': 'pyo',
  'みゃ': 'mya', 'みゅ': 'myu', 'みょ': 'myo',
  'りゃ': 'rya', 'りゅ': 'ryu', 'りょ': 'ryo',
  
  // Katakana Yōon
  'キャ': 'kya', 'キュ': 'kyu', 'キョ': 'kyo',
  'ギャ': 'gya', 'ギュ': 'gyu', 'ギョ': 'gyo',
  'シャ': 'sha', 'シュ': 'shu', 'ショ': 'sho',
  'ジャ': 'ja', 'ジュ': 'ju', 'ジョ': 'jo',
  'チャ': 'cha', 'チュ': 'chu', 'チョ': 'cho',
  'ニャ': 'nya', 'ニュ': 'nyu', 'ニョ': 'nyo',
  'ヒャ': 'hya', 'ヒュ': 'hyu', 'ヒョ': 'hyo',
  'ビャ': 'bya', 'ビュ': 'byu', 'ビょ': 'byo',
  'ピャ': 'pya', 'ピュ': 'pyu', 'ピョ': 'pyo',
  'ミャ': 'mya', 'ミュ': 'myu', 'ミョ': 'myo',
  'リャ': 'rya', 'リュ': 'ryu', 'リョ': 'ryo'
};

function kanaToRomaji(kana: string): string {
  let romaji = "";
  let i = 0;
  while (i < kana.length) {
    const char = kana[i];
    const nextChar = kana[i + 1];
    
    // Check for sokuon (double consonant)
    if (char === 'っ' || char === 'ッ') {
      if (nextChar) {
        let nextRomaji = "";
        if (i + 2 < kana.length) {
          const combo = nextChar + kana[i + 2];
          if (KANA_COMBINATIONS[combo]) {
            nextRomaji = KANA_COMBINATIONS[combo];
          }
        }
        if (!nextRomaji) {
          nextRomaji = KANA_TO_ROMAJI[nextChar] || "";
        }
        
        if (nextRomaji) {
          romaji += nextRomaji[0];
        }
      }
      i++;
      continue;
    }
    
    // Check for combinations (yōon)
    if (nextChar) {
      const combo = char + nextChar;
      if (KANA_COMBINATIONS[combo]) {
        romaji += KANA_COMBINATIONS[combo];
        i += 2;
        continue;
      }
    }
    
    // Single character
    romaji += KANA_TO_ROMAJI[char] || char;
    i++;
  }
  return romaji;
}

export function WordDisplay({ word }: { word: PracticeWord }) {
  const isJapanese = word.language === "ja";
  const romaji = isJapanese && word.reading ? kanaToRomaji(word.reading) : null;

  return (
    <article aria-label={`Palavra alvo: ${word.word}`} style={{ textAlign: "center", marginBottom: "1.5rem" }}>
      <h2 style={{ fontSize: "2.5rem", margin: 0, fontWeight: "bold", color: "#1a1a1a" }}>{word.word}</h2>
      {word.reading && (
        <p style={{ color: "#555", fontSize: "1.2rem", margin: "0.25rem 0 0 0", fontWeight: "medium" }}>
          {word.reading}
          {romaji && <span style={{ color: "#888", fontStyle: "italic", marginLeft: "0.5rem" }}>({romaji})</span>}
        </p>
      )}
      <p style={{ color: "#333", fontSize: "1.1rem", marginTop: "0.5rem" }}>{word.translation}</p>
      {word.pitchPattern && (
        <p aria-label={`Padrão de pitch accent: ${word.pitchPattern}`} style={{ fontSize: "0.9rem", color: "#777", margin: "0.25rem 0 0 0" }}>
          Pitch: <span style={{ fontWeight: "600" }}>{word.pitchPattern}</span>
        </p>
      )}
    </article>
  );
}
