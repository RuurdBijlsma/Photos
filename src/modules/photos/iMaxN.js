export default function iMaxN(arr, n = 3) {
    const indices = {};
    let entries = 0;
    let lowest = -Infinity;
    for (let i = 0; i < arr.length; i++) {
        let value = arr[i];
        if (value > lowest) {
            indices[value] = i;
            entries++;
            if (entries > n) {
                delete indices[lowest];
            }
            lowest = Math.min(...Object.keys(indices));
        }
    }
    return Object.values(indices);
}
