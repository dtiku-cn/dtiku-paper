$(function () {
    // ref: https://maxart2501.github.io/share-this/dist/sharers/notes.js
    rangy.init();
    var classApplierModule = rangy.modules.ClassApplier;
    if (rangy.supported && classApplierModule && classApplierModule.supported) {
        boldApplier = rangy.createClassApplier("selection-bold", {
            elementTagName: "b",
        });
        underlineApplier = rangy.createClassApplier("selection-line", {
            elementTagName: "u"
        });
        wavyApplier = rangy.createClassApplier("selection-wave", {
            elementTagName: "u"
        });
        highlightApplier = rangy.createClassApplier("selection-highlight", {
            elementTagName: "mark"
        });
    }
    var shareThisActions = [{
        name: 'bold',
        render: function () {
            return "<a title=\"加粗\" href=\"javascript:void(0)\">\n" +
                "<svg class=\"icon-svg icon-svg-sm\">\n" +
                "<use xlink:href=\"#ic-bold\"></use>\n" +
                "</svg>\n" +
                "</a>";
        },
        action: function (event) {
            event.preventDefault();
            event.stopPropagation();
            try {
                var range = window.getSelection().getRangeAt(0);
                var newNode = document.createElement("b");
                newNode.classList.add("selection-bold");
                range.surroundContents(newNode);
                newNode.focus();
            } catch (e) {
                boldApplier.toggleSelection();
            }
        }
    }, {
        name: 'underline',
        render: function () {
            return "<a title=\"下划线\" href=\"javascript:void(0)\">\n" +
                "<svg class=\"icon-svg icon-svg-sm\">\n" +
                "<use xlink:href=\"#ic-underline\"></use>\n" +
                "</svg>\n" +
                "</a>";
        },
        action: function (event) {
            event.preventDefault();
            event.stopPropagation();
            try {
                var range = window.getSelection().getRangeAt(0);
                var newNode = document.createElement("u");
                newNode.classList.add("selection-line");
                range.surroundContents(newNode);
                newNode.focus();
            } catch (e) {
                underlineApplier.toggleSelection();
            }
        }
    }, {
        name: 'wavy',
        render: function () {
            return "<a title=\"波浪线\" href=\"javascript:void(0)\">\n" +
                "<svg class=\"icon-svg icon-svg-sm\">\n" +
                "<use xlink:href=\"#ic-wave\"></use>\n" +
                "</svg>\n" +
                "</a>";
        },
        action: function (event) {
            event.preventDefault();
            event.stopPropagation();
            try {
                var range = window.getSelection().getRangeAt(0),
                    newNode = document.createElement("u");
                newNode.classList.add("selection-wave");
                range.surroundContents(newNode);
                newNode.focus();
            } catch (e) {
                wavyApplier.toggleSelection();
            }
        }
    }, {
        name: 'highlight',
        render: function () {
            return "<a title=\"高亮\" href=\"javascript:void(0)\">\n" +
                "<svg class=\"icon-svg icon-svg-sm\">\n" +
                "<use xlink:href=\"#ic-highlight\"></use>\n" +
                "</svg>\n" +
                "</a>";
        },
        action: function (event) {
            event.preventDefault();
            event.stopPropagation();
            try {
                var range = window.getSelection().getRangeAt(0),
                    newNode = document.createElement("mark");
                newNode.classList.add("selection-highlight");
                range.surroundContents(newNode);
                newNode.focus();
            } catch (e) {
                highlightApplier.toggleSelection();
            }
        }
    }];
    var shareThis = window.ShareThis,
        selectionShare = shareThis({
            selector: "#printcontent",
            sharers: shareThisActions
        });
    selectionShare.init();
    window.addEventListener("unload", function () {
        if ($(".paper-shenlun .material [class^='selection-']").size() > 0) {
            localStorage.setItem(location.pathname, $(".paper-shenlun .material").html());
        }
    });
    window.addEventListener("load", function () {
        var note = localStorage.getItem(location.pathname);
        if (note && note.length > 10) {
            $("<div class=\"modal fade\" id=\"loadMaterialNoteModal\" tabindex=\"-1\">\n" +
                "  <div class=\"modal-dialog\">\n" +
                "    <div class=\"modal-content\">\n" +
                "      <div class=\"modal-header\">\n" +
                "        <h5 class=\"modal-title\">加载笔记？</h5>\n" +
                "        <button type=\"button\" class=\"close\" data-dismiss=\"modal\" aria-label=\"Close\">\n" +
                "          <span aria-hidden=\"true\">&times;</span>\n" +
                "        </button>\n" +
                "      </div>\n" +
                "      <div class=\"modal-body\">发现上次的笔记，是否需要加载...</div>\n" +
                "      <div class=\"modal-footer\">\n" +
                "        <button type=\"button\" class=\"btn btn-secondary\" data-dismiss=\"modal\">忽略</button>\n" +
                "        <button type=\"button\" class=\"btn btn-primary\">加载笔记</button>\n" +
                "      </div>\n" +
                "    </div>\n" +
                "  </div>\n" +
                "</div>").appendTo(document.body);
            var $modal = $('#loadMaterialNoteModal');
            $modal.modal('show');
            $('#loadMaterialNoteModal .btn-primary').click(function () {
                $(".paper-shenlun .material").html(note);
                $modal.modal('hide').on('hidden.bs.modal', function () {
                    $modal.modal('dispose').remove();
                });
            });
        }
    });
});
// painter drawing-board
window.FloatButton = function (options) {
    options = Object.assign({
        parent: document.body,
        cssClass: [],
        buttonCss: [],
        menu: []
    }, options);

    function buildCss() {
        var s = options.menu.length;
        var ac = 0;
        var menuCss = options.menu.reverse().map(function (m, i) {
            var n = s - i;
            var rn = i + 1;
            if (m.style === 'always') {
                ac++;
                var t = rn * 4;
                var tc = ac * 4;
                return "#float-buttons.float-buttons-collapsed > div:nth-child(" + rn + "){transform:translateY(-" + tc + "rem) !important;}" +
                    "#float-buttons > div:nth-child(" + rn + "){transform:translateY(-" + t + "rem) !important;}";
            } else {
                var t = rn * 4;
                return "#float-buttons.float-buttons-collapsed > div:nth-child(" + rn + "){transform:translateY(0) !important;opacity: 0;}" +
                    "#float-buttons > div:nth-child(" + rn + "){transform:translateY(-" + t + "rem) !important;opacity: 1;}"
            }
        }).join("");
        return "<style>#float-buttons {position: fixed;bottom: 4rem;right: 3rem;z-index: 122;color:#007bff;}"+
            "#float-buttons .icon-svg{margin:2px;font-size:40px;}" +
            "#float-buttons > div {position: fixed;right: 0;z-index: 23;transition: all .2s;box-shadow: 3px 3px 6px 3px rgba(0, 0, 0, .3);}" +
            menuCss +
            "#float-buttons > div:hover {box-shadow: 3px 3px 6px 3px rgba(0, 0, 0, .45);}" +
            "#float-buttons > div:active {box-shadow: 2px 2px 4px 1px rgba(0, 0, 0, .6);}" +
            "#float-buttons > div:last-child {opacity: 1 !important;}</style>";
    }

    function buildHtml() {
        var menuHtml = options.menu.map(function (m, index) {
            return m.render ? "<div class='" + options.buttonCss.join(" ") + "' data-index='" + index + "'>" + m.render() + "</div>" : ""
        }).join("");
        return "<div id='float-buttons' class='float-buttons-collapsed " + options.cssClass.join(" ") + "'>" +
            menuHtml +
            "<div><svg class=\"icon-svg\"><use xlink:href=\"#ic-menu\"></use></svg></div>" +
            "</div>";
    }

    function bindClick() {
        $("#float-buttons>div").not(":last-child").click(function (e) {
            var index = $(this).data("index");
            var menu = options.menu[index];
            menu && menu.click && menu.click(e);
        });
        $("#float-buttons>div:last-child").click(function () {
            $("#float-buttons").toggleClass("float-buttons-collapsed");
        });
    }

    $(buildCss()).appendTo(document.head);
    $(buildHtml()).appendTo(options.parent);
    bindClick();
    var floatBtn = document.getElementById("float-buttons");
    return {
        target: floatBtn,
        parent: options.parent
    };
};

$(function () {
    var fontRem = Number(localStorage.getItem("paper-font-size")) || 1, $paper = $(".paper");
    $paper = $paper.size() === 0 ? $('#printcontent') : $paper;
    $paper.css("font-size", fontRem + "rem");
    window.floatButton = new FloatButton({
        cssClass: ['d-print-none'],
        menu: [{
            style: 'always',
            render: function () {
                return "<svg class='icon-svg' aria-hidden='true'>" +
                    "    <use xlink:href='#ic-top'></use>" +
                    "</svg>";
            },
            click: function () {
                window.scrollTo(0, 0);
            }
        }, $paper.size() ? {
            style: 'collapse',
            render: function () {
                return "<svg class='icon-svg' aria-hidden='true'>" +
                    "    <use xlink:href='#ic-zoom-in'></use>" +
                    "</svg>";
            },
            click: function () {
                fontRem += 0.1;
                fontRem = Math.min(fontRem, 3);
                $paper.css("font-size", fontRem + "rem");
                localStorage.setItem("paper-font-size", fontRem);
            }
        } : undefined, $paper.size() ? {
            style: 'collapse',
            render: function () {
                return "<svg class='icon-svg' aria-hidden='true'>" +
                    "    <use xlink:href='#ic-zoom-out'></use>" +
                    "</svg>";
            },
            click: function () {
                fontRem -= 0.1;
                fontRem = Math.max(fontRem, 0.8);
                $paper.css("font-size", fontRem + "rem");
                localStorage.setItem("paper-font-size", fontRem);
            }
        } : undefined, {
            style: location.pathname.startsWith("/paper/") ? 'always' : 'collapse',
            render: function () {
                return "<svg class='icon-svg' aria-hidden='true'>" +
                    "    <use xlink:href='#ic-pen'></use>" +
                    "</svg>";
            },
            click: function () {
                var hidden = $("#drawing-board").toggleClass('d-none').hasClass('d-none');
                $(".answer-paper-wrapper").css('bottom', hidden ? '0' : '-50px');
            }
        }].concat(window.floatButtonAddon).filter(function (m) {
            return !!m;
        })
    });
});