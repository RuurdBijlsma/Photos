// Paste this on bttv page console https://betterttv.com/emotes/trending

emotes = [...document.querySelectorAll('a')].filter(e => e.className.includes('emoteCard')).map(e => ({
    name: e.querySelector('div').innerText,
    url: e.querySelector('img').src,
}));

JSON.stringify(emotes);