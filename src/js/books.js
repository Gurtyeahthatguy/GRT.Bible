/** The 73 books of the canon, their names and the aliases the reference field accepts. */

// osis, testament, deuterocanonical, [English, abbr], [Italian, abbr], [Latin, abbr], extra aliases
const TABLE = [
  ['Gen', 'OT', 0, ['Genesis', 'Gen'], ['Genesi', 'Gen'], ['Genesis', 'Gen'], 'gn ge'],
  ['Exod', 'OT', 0, ['Exodus', 'Exod'], ['Esodo', 'Es'], ['Exodus', 'Ex'], 'exo'],
  ['Lev', 'OT', 0, ['Leviticus', 'Lev'], ['Levitico', 'Lv'], ['Leviticus', 'Lev'], ''],
  ['Num', 'OT', 0, ['Numbers', 'Num'], ['Numeri', 'Nm'], ['Numeri', 'Num'], 'nb'],
  ['Deut', 'OT', 0, ['Deuteronomy', 'Deut'], ['Deuteronomio', 'Dt'], ['Deuteronomium', 'Deut'], 'deu'],
  ['Josh', 'OT', 0, ['Joshua', 'Josh'], ['Giosuè', 'Gs'], ['Iosue', 'Ios'], 'jos josue'],
  ['Judg', 'OT', 0, ['Judges', 'Judg'], ['Giudici', 'Gdc'], ['Iudicum', 'Idc'], 'jdg jgs'],
  ['Ruth', 'OT', 0, ['Ruth', 'Ruth'], ['Rut', 'Rt'], ['Ruth', 'Ruth'], 'rth'],
  ['1Sam', 'OT', 0, ['1 Samuel', '1 Sam'], ['1 Samuele', '1Sam'], ['I Samuelis', '1Sam'], '1sm'],
  ['2Sam', 'OT', 0, ['2 Samuel', '2 Sam'], ['2 Samuele', '2Sam'], ['II Samuelis', '2Sam'], '2sm'],
  ['1Kgs', 'OT', 0, ['1 Kings', '1 Kgs'], ['1 Re', '1Re'], ['I Regum', '1Reg'], '1ki 1kg'],
  ['2Kgs', 'OT', 0, ['2 Kings', '2 Kgs'], ['2 Re', '2Re'], ['II Regum', '2Reg'], '2ki 2kg'],
  ['1Chr', 'OT', 0, ['1 Chronicles', '1 Chr'], ['1 Cronache', '1Cr'], ['I Paralipomenon', '1Par'], '1ch 1paralipomeni'],
  ['2Chr', 'OT', 0, ['2 Chronicles', '2 Chr'], ['2 Cronache', '2Cr'], ['II Paralipomenon', '2Par'], '2ch 2paralipomeni'],
  ['Ezra', 'OT', 0, ['Ezra', 'Ezra'], ['Esdra', 'Esd'], ['Esdrae', 'Esd'], 'ezr'],
  ['Neh', 'OT', 0, ['Nehemiah', 'Neh'], ['Neemia', 'Ne'], ['Nehemiae', 'Neh'], ''],
  ['Tob', 'OT', 1, ['Tobit', 'Tob'], ['Tobia', 'Tb'], ['Tobiae', 'Tob'], 'tobias'],
  ['Jdt', 'OT', 1, ['Judith', 'Jdt'], ['Giuditta', 'Gdt'], ['Iudith', 'Idt'], 'jdth'],
  ['Esth', 'OT', 0, ['Esther', 'Esth'], ['Ester', 'Est'], ['Esther', 'Est'], ''],
  ['1Macc', 'OT', 1, ['1 Maccabees', '1 Macc'], ['1 Maccabei', '1Mac'], ['I Machabaeorum', '1Mac'], '1mc 1ma'],
  ['2Macc', 'OT', 1, ['2 Maccabees', '2 Macc'], ['2 Maccabei', '2Mac'], ['II Machabaeorum', '2Mac'], '2mc 2ma'],
  ['Job', 'OT', 0, ['Job', 'Job'], ['Giobbe', 'Gb'], ['Iob', 'Iob'], 'jb'],
  ['Ps', 'OT', 0, ['Psalms', 'Ps'], ['Salmi', 'Sal'], ['Psalmi', 'Ps'], 'salmo sl psalm psalmus pss psa'],
  ['Prov', 'OT', 0, ['Proverbs', 'Prov'], ['Proverbi', 'Pr'], ['Proverbia', 'Prov'], 'prv pro'],
  ['Eccl', 'OT', 0, ['Ecclesiastes', 'Eccl'], ['Qoelet', 'Qo'], ['Ecclesiastes', 'Eccle'], 'qohelet ecclesiaste eccles'],
  ['Song', 'OT', 0, ['Song of Songs', 'Song'], ['Cantico dei Cantici', 'Ct'], ['Canticum Canticorum', 'Cant'], 'cantico canticum songofsolomon sg'],
  ['Wis', 'OT', 1, ['Wisdom', 'Wis'], ['Sapienza', 'Sap'], ['Sapientia', 'Sap'], 'ws'],
  ['Sir', 'OT', 1, ['Sirach', 'Sir'], ['Siracide', 'Sir'], ['Ecclesiasticus', 'Eccli'], 'ecclesiastico ecclus'],
  ['Isa', 'OT', 0, ['Isaiah', 'Isa'], ['Isaia', 'Is'], ['Isaias', 'Is'], ''],
  ['Jer', 'OT', 0, ['Jeremiah', 'Jer'], ['Geremia', 'Ger'], ['Ieremias', 'Ier'], 'jr'],
  ['Lam', 'OT', 0, ['Lamentations', 'Lam'], ['Lamentazioni', 'Lam'], ['Lamentationes', 'Lam'], ''],
  ['Bar', 'OT', 1, ['Baruch', 'Bar'], ['Baruc', 'Bar'], ['Baruch', 'Bar'], ''],
  ['Ezek', 'OT', 0, ['Ezekiel', 'Ezek'], ['Ezechiele', 'Ez'], ['Ezechiel', 'Ez'], 'ezk'],
  ['Dan', 'OT', 0, ['Daniel', 'Dan'], ['Daniele', 'Dn'], ['Daniel', 'Dan'], ''],
  ['Hos', 'OT', 0, ['Hosea', 'Hos'], ['Osea', 'Os'], ['Osee', 'Os'], ''],
  ['Joel', 'OT', 0, ['Joel', 'Joel'], ['Gioele', 'Gl'], ['Ioel', 'Ioel'], 'jl'],
  ['Amos', 'OT', 0, ['Amos', 'Amos'], ['Amos', 'Am'], ['Amos', 'Am'], ''],
  ['Obad', 'OT', 0, ['Obadiah', 'Obad'], ['Abdia', 'Abd'], ['Abdias', 'Abd'], 'ob'],
  ['Jonah', 'OT', 0, ['Jonah', 'Jonah'], ['Giona', 'Gn'], ['Ionas', 'Ion'], 'jon jonas'],
  ['Mic', 'OT', 0, ['Micah', 'Mic'], ['Michea', 'Mi'], ['Michaeas', 'Mich'], ''],
  ['Nah', 'OT', 0, ['Nahum', 'Nah'], ['Naum', 'Na'], ['Nahum', 'Nah'], ''],
  ['Hab', 'OT', 0, ['Habakkuk', 'Hab'], ['Abacuc', 'Ab'], ['Habacuc', 'Hab'], ''],
  ['Zeph', 'OT', 0, ['Zephaniah', 'Zeph'], ['Sofonia', 'Sof'], ['Sophonias', 'Soph'], 'zep'],
  ['Hag', 'OT', 0, ['Haggai', 'Hag'], ['Aggeo', 'Ag'], ['Aggaeus', 'Agg'], ''],
  ['Zech', 'OT', 0, ['Zechariah', 'Zech'], ['Zaccaria', 'Zc'], ['Zacharias', 'Zach'], 'zec'],
  ['Mal', 'OT', 0, ['Malachi', 'Mal'], ['Malachia', 'Ml'], ['Malachias', 'Mal'], ''],
  ['Matt', 'NT', 0, ['Matthew', 'Matt'], ['Matteo', 'Mt'], ['Matthaeus', 'Mt'], 'mat'],
  ['Mark', 'NT', 0, ['Mark', 'Mark'], ['Marco', 'Mc'], ['Marcus', 'Mc'], 'mk mr'],
  ['Luke', 'NT', 0, ['Luke', 'Luke'], ['Luca', 'Lc'], ['Lucas', 'Lc'], 'lk luk'],
  ['John', 'NT', 0, ['John', 'John'], ['Giovanni', 'Gv'], ['Ioannes', 'Io'], 'jn joh jhn'],
  ['Acts', 'NT', 0, ['Acts', 'Acts'], ['Atti degli Apostoli', 'At'], ['Actus Apostolorum', 'Act'], 'atti actus'],
  ['Rom', 'NT', 0, ['Romans', 'Rom'], ['Romani', 'Rm'], ['Ad Romanos', 'Rom'], 'ro'],
  ['1Cor', 'NT', 0, ['1 Corinthians', '1 Cor'], ['1 Corinzi', '1Cor'], ['I ad Corinthios', '1Cor'], '1co'],
  ['2Cor', 'NT', 0, ['2 Corinthians', '2 Cor'], ['2 Corinzi', '2Cor'], ['II ad Corinthios', '2Cor'], '2co'],
  ['Gal', 'NT', 0, ['Galatians', 'Gal'], ['Galati', 'Gal'], ['Ad Galatas', 'Gal'], ''],
  ['Eph', 'NT', 0, ['Ephesians', 'Eph'], ['Efesini', 'Ef'], ['Ad Ephesios', 'Eph'], ''],
  ['Phil', 'NT', 0, ['Philippians', 'Phil'], ['Filippesi', 'Fil'], ['Ad Philippenses', 'Phil'], 'php'],
  ['Col', 'NT', 0, ['Colossians', 'Col'], ['Colossesi', 'Col'], ['Ad Colossenses', 'Col'], ''],
  ['1Thess', 'NT', 0, ['1 Thessalonians', '1 Thess'], ['1 Tessalonicesi', '1Ts'], ['I ad Thessalonicenses', '1Thess'], '1th'],
  ['2Thess', 'NT', 0, ['2 Thessalonians', '2 Thess'], ['2 Tessalonicesi', '2Ts'], ['II ad Thessalonicenses', '2Thess'], '2th'],
  ['1Tim', 'NT', 0, ['1 Timothy', '1 Tim'], ['1 Timoteo', '1Tm'], ['I ad Timotheum', '1Tim'], '1ti'],
  ['2Tim', 'NT', 0, ['2 Timothy', '2 Tim'], ['2 Timoteo', '2Tm'], ['II ad Timotheum', '2Tim'], '2ti'],
  ['Titus', 'NT', 0, ['Titus', 'Titus'], ['Tito', 'Tt'], ['Ad Titum', 'Tit'], ''],
  ['Phlm', 'NT', 0, ['Philemon', 'Phlm'], ['Filemone', 'Fm'], ['Ad Philemonem', 'Philem'], 'phm'],
  ['Heb', 'NT', 0, ['Hebrews', 'Heb'], ['Ebrei', 'Eb'], ['Ad Hebraeos', 'Hebr'], ''],
  ['Jas', 'NT', 0, ['James', 'Jas'], ['Giacomo', 'Gc'], ['Iacobi', 'Iac'], 'jm'],
  ['1Pet', 'NT', 0, ['1 Peter', '1 Pet'], ['1 Pietro', '1Pt'], ['I Petri', '1Pet'], '1pe'],
  ['2Pet', 'NT', 0, ['2 Peter', '2 Pet'], ['2 Pietro', '2Pt'], ['II Petri', '2Pet'], '2pe'],
  ['1John', 'NT', 0, ['1 John', '1 John'], ['1 Giovanni', '1Gv'], ['I Ioannis', '1Io'], '1jn'],
  ['2John', 'NT', 0, ['2 John', '2 John'], ['2 Giovanni', '2Gv'], ['II Ioannis', '2Io'], '2jn'],
  ['3John', 'NT', 0, ['3 John', '3 John'], ['3 Giovanni', '3Gv'], ['III Ioannis', '3Io'], '3jn'],
  ['Jude', 'NT', 0, ['Jude', 'Jude'], ['Giuda', 'Gd'], ['Iudae', 'Iud'], 'jud'],
  ['Rev', 'NT', 0, ['Revelation', 'Rev'], ['Apocalisse', 'Ap'], ['Apocalypsis', 'Apoc'], 'apocalypse rv'],
];

export const BOOKS = TABLE.map(([osis, testament, deutero, en, it, la, extra], i) => ({
  index: i + 1,
  osis,
  testament,
  deutero: deutero === 1,
  names: { en, it, la },
  extra: extra ? extra.split(' ') : [],
}));

export const BY_OSIS = new Map(BOOKS.map((b) => [b.osis, b]));

/** Lower case, no accents, no dots or spaces. */
export function foldKey(text) {
  return text
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/[.\s]/g, '');
}

/** Name and abbreviation in the language of a module; Greek and Hebrew use Latin. */
export function nameIn(osis, language) {
  const book = BY_OSIS.get(osis);
  if (!book) return [osis, osis];
  if (language === 'en') return book.names.en;
  if (language === 'it') return book.names.it;
  return book.names.la;
}

function buildAliases() {
  const map = new Map();
  const add = (alias, osis) => {
    const key = foldKey(alias);
    if (!key) return;
    const list = map.get(key) || [];
    if (!list.includes(osis)) list.push(osis);
    map.set(key, list);
  };
  for (const book of BOOKS) {
    for (const [name, abbr] of Object.values(book.names)) {
      add(name, book.osis);
      add(abbr, book.osis);
    }
    add(book.osis, book.osis);
    for (const extra of book.extra) add(extra, book.osis);
  }
  return map;
}

/** Folded alias to the books it can mean. More than one book means ask. */
export const ALIASES = buildAliases();
