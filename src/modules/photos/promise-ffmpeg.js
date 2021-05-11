import util from 'util'
import path from 'path'
import ffmpeg from "fluent-ffmpeg";

ffmpeg.ffprobe = util.promisify(ffmpeg.ffprobe)

ffmpeg.screenshot = ({file, output, timeStamp = 0, width = 720, height = null, signal = null}) => {

    return new Promise(((resolve, reject) => {
        if (width === null && height === null)
            return reject("Width and height can't both be null");
        let command = ffmpeg(file)
            .on('end', () => resolve(output.replace(/\\/g, '/')))
            .on('error', reject)
            .screenshots({
                timestamps: [timeStamp],
                size: `${width ?? '?'}x${height ?? '?'}`,
                filename: path.basename(output),
                folder: path.dirname(output),
            });
        signal?.addEventListener?.('abort', () => command.kill());
    }))
}

export default ffmpeg;
