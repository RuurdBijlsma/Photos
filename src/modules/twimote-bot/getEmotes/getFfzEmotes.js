// Paste this on ffz page console https://www.frankerfacez.com/emoticons/?q=pause&sort=count-desc&days=0

emotes = [...document.querySelectorAll('.selectable')].map(row=>({
    name: row.querySelector('.emote-name>a').innerText,
    url: row.querySelector('img.emoticon').src,
}));
emotes.forEach(e => e.url = e.url.substr(0, e.url.length - 1))

async function loadImg(url){
    return new Promise(resolve=>{
        let img = new Image();
        img.src = url;
        img.onload = ()=>{
            resolve(img);
        }
        img.onerror = ()=>{
            resolve(false);
        }
    });
}

let imgs = await Promise.all(emotes.map(e=>loadImg(e.url+4)));
for(let i=0; i<imgs.length;i++){
    if(imgs[i] !== false){
        emotes[i].url += 4;
    }else{
        emotes[i].url += 1;
    }
}

JSON.stringify(emotes);