import * as tf from '@tensorflow/tfjs-node'
import fs from 'fs'
import {EOL} from "os";
import iMaxN from './iMaxN.js';
import WordNet from 'node-wordnet'
import path from "path";

const wordnet = new WordNet();
let model, labels, syns;

async function initClassifier() {
    labels = await fs.promises.readFile('./res/photos/classifier/labels.txt').then(f => f.toString().split(EOL));
    syns = await fs.promises.readFile('./res/photos/classifier/syns.txt').then(f => f.toString().split(EOL));

    // https://tfhub.dev/google/imagenet/inception_resnet_v2/classification/5
    let modelPath = path.resolve('./res/photos/classifier/inceptionresnet/model.json');
    if (process.platform === 'win32')
        modelPath = modelPath.substr(3);
    model = await tf.loadGraphModel(`file:///${modelPath}`);
}

const ready = initClassifier();

export default async function classify(imagePath, nLabels = 3) {
    await ready;

    let imageBuffer = await fs.promises.readFile(imagePath);
    let image;
    try {
        image = tf.node.decodeImage(imageBuffer);
    } catch (e) {
        console.warn("Can't decode image: ", imagePath);
        throw new Error(e);
    }
    const targetSize = [299, 299];
    let resizedImage = tf.image.resizeBilinear(image, targetSize);
    image.dispose();
    let reshapedImage = tf.reshape(resizedImage, [-1, ...targetSize, 3]);
    resizedImage.dispose();
    let normalizedImage = reshapedImage.div(tf.scalar(255));
    reshapedImage.dispose();

    let result = model.predict(normalizedImage, {batchSize: 1});
    normalizedImage.dispose();
    let predicted = await result.array().then(d => d[0]);
    result.dispose();
    let maxIndices = iMaxN(predicted, nLabels);

    let resultArr = [];
    for (let i of maxIndices) {
        let {labels, glossaries} = await getLabelWord(i);
        resultArr.push({
            logits: predicted[i],
            labels, glossaries
        });
    }
    let maxLogits = Math.max(...resultArr.map(r => r.logits));
    return resultArr
        .map(r => ({
            confidence: r.logits / maxLogits,
            labels: r.labels,
            glossaries: r.glossaries
        }))
        .sort((a, b) => b.confidence - a.confidence)
}

async function getLabelWord(idx) {
    try {
        let ss = syns[idx];
        let word = await wordnet.getAsync(+ss.substr(1), ss.substring(0, 1));
        let parents = [];
        let parent = word;
        while (true) {
            let pointer = parent.ptrs.find(p => p.pointerSymbol === '@');
            if (!pointer)
                break;
            parent = await wordnet.getAsync(pointer.synsetOffset, pointer.pos);
            if (parent.lexFilenum !== word.lexFilenum)
                break;
            parents.push(parent);
        }
        let hierarchy = [word, ...parents].map((word, index)=>({...parseWord(word), index}));
        let labels = hierarchy.flatMap(w => w.names.map(text=>({text, level: w.index})));
        let glossaries = hierarchy.map(w => ({text: w.glossary, level: w.index}));
        return {labels, glossaries};
    } catch (e) {
        return {labels: [labels[idx]], glossaries: []};
    }
}

function parseWord(word) {
    return {
        names: [...new Set([...word.synonyms, word.lemma])].map(w => w.replace(/_/g, ' ')),
        synset: word.pos + word.synsetOffset,
        glossary: word.gloss.trim(),
    };
}

// classify('./res/photos/photos/20170727_204153.jpg').then(c => {
//     console.log(c);
// })
