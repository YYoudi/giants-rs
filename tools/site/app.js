'use strict';
const $ = selector => document.querySelector(selector);
const el = (tag, text, className) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
};
function filterRoadmap() {
  let count = 0;
  const query = $('#search').value.trim().toLowerCase();
  for (const group of document.querySelectorAll('.roadmap-group')) {
    let shown = 0;
    for (const row of group.querySelectorAll('.entry')) {
      const text = `${group.querySelector('.group-heading').textContent} ${row.textContent}`.toLowerCase();
      row.hidden = !text.includes(query) || ($('#status').value !== '' && row.dataset.status !== $('#status').value);
      if (!row.hidden) shown++;
    }
    group.hidden = shown === 0;
    count += shown;
  }
  $('#results').textContent = `${count} roadmap entries shown (including technical foundations)`;
  $('#empty').hidden = count !== 0;
  const foundations = $('.foundations');
  if (query || $('#status').value) foundations.open = !!foundations.querySelector('.entry:not([hidden])');
}
$('#search').addEventListener('input', filterRoadmap);
$('#status').addEventListener('change', filterRoadmap);
$('#reset').addEventListener('click', () => {
  $('#search').value = ''; $('#status').value = ''; filterRoadmap();
});
filterRoadmap();
let comparisons = [], selected = [], displayed = 0;
function imagePath(path) {
  if (!/^media\/[a-f0-9]{64}\.webp$/.test(path)) throw Error('Invalid gallery image');
  return path;
}
function picture(path, label, eager) {
  const figure = el('figure'), link = el('a'), img = el('img');
  link.href = imagePath(path); link.target = '_blank'; link.rel = 'noopener';
  link.setAttribute('aria-label', `Open image: ${label}`);
  img.src = imagePath(path); img.alt = label; img.loading = eager ? 'eager' : 'lazy'; img.decoding = 'async';
  link.append(img); figure.append(link, el('figcaption', label));
  return figure;
}
function loadMore() {
  const page = selected.slice(displayed, displayed + 6);
  for (const [index, row] of page.entries()) {
    const article = el('article', undefined, 'comparison'), heading = el('div', undefined, 'comparison-head');
    heading.append(el('p', row.level, 'eyebrow'), el('h3', row.title));
    const pair = el('div', undefined, 'pair');
    pair.append(picture(row.left, row.left_label, displayed === 0 && index === 0), picture(row.right, row.right_label, displayed === 0 && index === 0));
    const notes = el('div', undefined, 'comparison-notes'), detail = el('details');
    notes.append(el('strong', row.verification || 'Fidelity unverified'));
    detail.append(el('summary', `Context & limits · archive revision ${(row.commit || 'unrecorded').slice(0, 7)}`), el('p', row.alignment || 'No additional context recorded.'));
    notes.append(detail); article.append(heading, pair, notes); $('#gallery').append(article);
  }
  displayed += page.length;
  $('#gallery-count').textContent = `${selected.length} of ${comparisons.length} comparisons match · ${displayed} displayed · newest archive entries first`;
  $('#load-more').hidden = displayed >= selected.length;
}
function filterGallery() {
  const query = $('#gallery-search').value.trim().toLowerCase();
  selected = comparisons.filter(row => (!$('#level').value || row.level === $('#level').value) &&
    (!$('#event').value || row.event === $('#event').value) &&
    `${row.title} ${row.level} ${row.event} ${row.commit || ''} ${row.left_label} ${row.right_label}`.toLowerCase().includes(query));
  displayed = 0; $('#gallery').replaceChildren(); loadMore();
  if (!selected.length) $('#gallery').append(el('p', 'No comparisons match these filters.'));
}
async function initGallery() {
  const response = await fetch('gallery.json');
  if (!response.ok) throw Error(`Gallery unavailable (${response.status})`);
  const data = await response.json();
  comparisons = [...data.comparisons].reverse();
  for (const key of ['level', 'event']) {
    for (const value of [...new Set(comparisons.map(row => row[key]).filter(Boolean))].sort()) {
      const option = el('option', value); option.value = value; $('#' + key).append(option);
    }
    $('#' + key).addEventListener('change', filterGallery);
  }
  $('#gallery-search').addEventListener('input', filterGallery);
  $('#load-more').addEventListener('click', loadMore);
  const hero = comparisons.find(row => row.title === 'Owned Mecc jet: normal thrust and boost');
  if (hero) {
    const img = el('img'); img.src = imagePath(hero.right); img.alt = ''; img.decoding = 'async';
    $('#hero-image').append(img, el('figcaption', 'Historical Bevy development render · fidelity unverified'));
    $('#hero-image').hidden = false;
  }
  filterGallery();
}
initGallery().catch(() => { $('#gallery-count').textContent = 'The comparison archive could not be loaded. Reload to retry.'; });
