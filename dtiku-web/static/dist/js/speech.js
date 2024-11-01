$(function () {
    var SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;

    if (SpeechRecognition) {
        function newRecognition() {
            var r = new SpeechRecognition();
            r.continuous = false;
            r.lang = 'zh-CN';
            r.interimResults = false;
            r.maxAlternatives = 1;
            r.onresult = onSpeechResult;
            r.onspeechend = onSpeechEnd;
            r.onerror = onSpeechError;
            return r;
        }

        $('.input-group input[type="text"].speech-input').each(function (i) {
            var $this = $(this);
            $("<div class=\"input-group-append\">" +
                "<button class=\"btn btn-speech d-flex align-items-center border-0\" title='语音输入'>" +
                "<svg class=\"icon-svg icon-svg-md\"><use xlink:href=\"#ic-voice\"></use></svg>" +
                "</button></div>").insertAfter($this).click(function () {
                var recognition = newRecognition(),
                    $target = $(this).find('.btn-speech');
                recognition.start();
                $target.addClass('text-success');
                recognition.targetElement = $target;
                recognition.targetInput = $this;
                console.log('Ready to receive a speech command.');
            })
        });

        function onSpeechResult(e) {
            e.target.targetInput && e.target.targetInput.val(e.results[0][0].transcript);
            e.target.targetElement && e.target.targetElement.removeClass('text-success text-danger').find('svg.icon-svg').html('<use xlink:href=\"#ic-voice\"></use>');
            console.log('confidence', e.results[0][0].confidence);
        }

        function onSpeechEnd(e) {
            e.target.stop();
            console.log("speech end");
        }

        function onSpeechError(e) {
            if (e.target.targetElement) {
                $(e.target.targetElement).addClass('text-danger').find('svg.icon-svg').html('<use xlink:href=\"#ic-muted\"></use>');
            }
            console.log("speech error");
        }
    }

    if (window.speechSynthesis) {
        var synth = window.speechSynthesis, chineseVoices = synth.getVoices().filter(function (v) {
            return v.lang.startsWith('zh')
        });

        function onSynthesisEnd(e) {
            $(".speaker-control").toggleClass("d-none", true);
            console.log('SpeechSynthesisUtterance.onend');
        }

        function onSynthesisError(e) {
            console.error('SpeechSynthesisUtterance.onerror');
        }

        $("<div class='speaker-control position-absolute d-none'>" +
            "</div>").appendTo($(document.body));

        $(".question").add(".solution").add(".material").addClass("position-relative").each(function () {
            var $this = $(this);
            $("<div class='speaker position-absolute d-print-none' style='top:-.7em;bottom:0;left:-6em;width:2em'>" +
                "<button class=\"position-sticky btn btn-speaker d-flex align-items-center justify-content-center rounded-circle border-0\" title='朗读' style='width:3em;height:3em;top:0;left:0;opacity:.01'>" +
                "<svg class=\"icon-svg icon-svg-md\"><use xlink:href=\"#ic-speaker\"></use></svg>" +
                "</button></div>").appendTo($this).click(function () {
                var utterThis = new SpeechSynthesisUtterance($this.text());
                utterThis.onend = onSynthesisEnd;
                utterThis.onerror = onSynthesisError;
                utterThis.voice = chineseVoices[0];
                utterThis.pitch = 1;
                utterThis.rate = 1;
                synth.cancel();
                synth.speak(utterThis);
                $(".speaker-control").toggleClass("d-none", false);
            });
        }).hover(function () {
            $(this).find("div.speaker>button").css('opacity', '1')
        }, function () {
            $(this).find("div.speaker>button").css('opacity', '.01')
        });
    }

});
