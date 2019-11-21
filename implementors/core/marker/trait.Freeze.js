(function() {var implementors = {};
implementors["failure"] = [{text:"impl !Freeze for <a class=\"struct\" href=\"failure/struct.Backtrace.html\" title=\"struct failure::Backtrace\">Backtrace</a>",synthetic:true,types:["failure::backtrace::Backtrace"]},{text:"impl&lt;E&gt; Freeze for <a class=\"struct\" href=\"failure/struct.Compat.html\" title=\"struct failure::Compat\">Compat</a>&lt;E&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;E: Freeze,&nbsp;</span>",synthetic:true,types:["failure::compat::Compat"]},{text:"impl&lt;D&gt; !Freeze for <a class=\"struct\" href=\"failure/struct.Context.html\" title=\"struct failure::Context\">Context</a>&lt;D&gt;",synthetic:true,types:["failure::context::Context"]},{text:"impl&lt;T&gt; !Freeze for <a class=\"struct\" href=\"failure/struct.SyncFailure.html\" title=\"struct failure::SyncFailure\">SyncFailure</a>&lt;T&gt;",synthetic:true,types:["failure::sync_failure::SyncFailure"]},{text:"impl Freeze for <a class=\"struct\" href=\"failure/struct.Error.html\" title=\"struct failure::Error\">Error</a>",synthetic:true,types:["failure::error::Error"]},{text:"impl&lt;'f&gt; Freeze for <a class=\"struct\" href=\"failure/struct.Causes.html\" title=\"struct failure::Causes\">Causes</a>&lt;'f&gt;",synthetic:true,types:["failure::Causes"]},];
implementors["izanami"] = [{text:"impl&lt;'a&gt; Freeze for <a class=\"struct\" href=\"izanami/context/struct.Context.html\" title=\"struct izanami::context::Context\">Context</a>&lt;'a&gt;",synthetic:true,types:["izanami::context::Context"]},{text:"impl&lt;T&gt; !Freeze for <a class=\"struct\" href=\"izanami/struct.Launcher.html\" title=\"struct izanami::Launcher\">Launcher</a>&lt;T&gt;",synthetic:true,types:["izanami::launcher::Launcher"]},{text:"impl&lt;T&gt; Freeze for <a class=\"struct\" href=\"izanami/body/struct.IntoBufStream.html\" title=\"struct izanami::body::IntoBufStream\">IntoBufStream</a>&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: Freeze,&nbsp;</span>",synthetic:true,types:["izanami::body::IntoBufStream"]},{text:"impl !Freeze for <a class=\"struct\" href=\"izanami/body/struct.Body.html\" title=\"struct izanami::body::Body\">Body</a>",synthetic:true,types:["izanami::body::Body"]},{text:"impl !Freeze for <a class=\"struct\" href=\"izanami/body/struct.Data.html\" title=\"struct izanami::body::Data\">Data</a>",synthetic:true,types:["izanami::body::Data"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/body/struct.Error.html\" title=\"struct izanami::body::Error\">Error</a>",synthetic:true,types:["izanami::body::Error"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/context/struct.CookieParseError.html\" title=\"struct izanami::context::CookieParseError\">CookieParseError</a>",synthetic:true,types:["izanami::context::CookieParseError"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/context/struct.WsHandshakeError.html\" title=\"struct izanami::context::WsHandshakeError\">WsHandshakeError</a>",synthetic:true,types:["izanami::context::WsHandshakeError"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/error/struct.BoxedStdCompat.html\" title=\"struct izanami::error::BoxedStdCompat\">BoxedStdCompat</a>",synthetic:true,types:["izanami::error::BoxedStdCompat"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/error/struct.Error.html\" title=\"struct izanami::error::Error\">Error</a>",synthetic:true,types:["izanami::error::Error"]},{text:"impl&lt;T&gt; Freeze for <a class=\"struct\" href=\"izanami/localmap/struct.LocalKey.html\" title=\"struct izanami::localmap::LocalKey\">LocalKey</a>&lt;T&gt;",synthetic:true,types:["izanami::localmap::LocalKey"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/localmap/struct.KeyId.html\" title=\"struct izanami::localmap::KeyId\">KeyId</a>",synthetic:true,types:["izanami::localmap::KeyId"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/localmap/struct.LocalMap.html\" title=\"struct izanami::localmap::LocalMap\">LocalMap</a>",synthetic:true,types:["izanami::localmap::LocalMap"]},{text:"impl&lt;'a, K&gt; Freeze for <a class=\"struct\" href=\"izanami/localmap/struct.OccupiedEntry.html\" title=\"struct izanami::localmap::OccupiedEntry\">OccupiedEntry</a>&lt;'a, K&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;K: Freeze,&nbsp;</span>",synthetic:true,types:["izanami::localmap::OccupiedEntry"]},{text:"impl&lt;'a, K&gt; Freeze for <a class=\"struct\" href=\"izanami/localmap/struct.VacantEntry.html\" title=\"struct izanami::localmap::VacantEntry\">VacantEntry</a>&lt;'a, K&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;K: Freeze,&nbsp;</span>",synthetic:true,types:["izanami::localmap::VacantEntry"]},{text:"impl&lt;'a, K&gt; Freeze for <a class=\"enum\" href=\"izanami/localmap/enum.Entry.html\" title=\"enum izanami::localmap::Entry\">Entry</a>&lt;'a, K&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;K: Freeze,&nbsp;</span>",synthetic:true,types:["izanami::localmap::Entry"]},{text:"impl&lt;F&gt; Freeze for <a class=\"struct\" href=\"izanami/rt/struct.BlockingSection.html\" title=\"struct izanami::rt::BlockingSection\">BlockingSection</a>&lt;F&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;F: Freeze,&nbsp;</span>",synthetic:true,types:["izanami::rt::BlockingSection"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/ws/struct.WebSocket.html\" title=\"struct izanami::ws::WebSocket\">WebSocket</a>",synthetic:true,types:["izanami::ws::WebSocket"]},{text:"impl Freeze for <a class=\"struct\" href=\"izanami/ws/struct.Error.html\" title=\"struct izanami::ws::Error\">Error</a>",synthetic:true,types:["izanami::ws::Error"]},];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()
