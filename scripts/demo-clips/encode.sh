#!/bin/sh
# encode.sh <dir with the .mkv masters> <site/video out dir> [ffmpeg]
# Two sizes per clip (1200 and 720 wide), H.264 yuv420p, no audio, faststart, plus a WebP
# poster of the last frame (also the reduced-motion still). CLIPS="a b" encodes only those.
IN=$1; OUT=$2; FF=${3:-ffmpeg}
CRF=${CRF:-28}
CLIPS=${CLIPS:-"select annotate redact history steps magnify measure beautify sensitive picker multi"}
enc() { # name variant filter
  case " $CLIPS " in *" $1 "*) ;; *) return ;; esac
  "$FF" -y -hide_banner -loglevel error -i "$IN/$1.mkv" \
    -vf "$3,scale=out_color_matrix=bt709:out_range=tv,format=yuv420p" \
    -c:v libx264 -preset veryslow -tune animation -crf "$CRF" -g 300 \
    -colorspace bt709 -color_primaries bt709 -color_trc bt709 \
    -movflags +faststart -an "$OUT/$1-$2.mp4"
  "$FF" -y -hide_banner -loglevel error -sseof -0.2 -i "$IN/$1.mkv" -frames:v 1 \
    -vf "$3" -c:v libwebp -quality 80 "$OUT/$1-$2.webp"
}
S=flags=lanczos
DASH="crop=1160:850:760:150"
enc select 1200 "scale=1200:-2:$S"
enc select 720 "$DASH,scale=720:-2:$S"
enc annotate 1200 "scale=1200:-2:$S"
enc annotate 720 "$DASH,scale=720:-2:$S"
enc redact 1200 "crop=1200:540:100:640"
enc redact 720 "crop=1200:540:100:640,scale=720:-2:$S"
enc history 1200 "crop=1920:840:0:0,scale=1200:-2:$S"
enc history 720 "crop=1920:840:0:0,scale=720:-2:$S"
# Tool clips (0.1.1 set).
for c in steps magnify measure; do
  enc $c 1200 "scale=1200:-2:$S"
  enc $c 720 "$DASH,scale=720:-2:$S"
done
enc picker 1200 "scale=1200:-2:$S"
enc picker 720 "crop=1240:1000:640:20,scale=720:-2:$S"
enc beautify 1200 "scale=1200:-2:$S"
enc beautify 720 "crop=1700:1060:40:20,scale=720:-2:$S"
enc sensitive 1200 "crop=1720:500:100:640,scale=1200:-2:$S"
enc sensitive 720 "crop=1720:500:100:640,scale=720:-2:$S"
enc multi 1200 "scale=1200:-2:$S"
enc multi 720 "crop=1300:760:560:240,scale=720:-2:$S"
ls -l "$OUT"
