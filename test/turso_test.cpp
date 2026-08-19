////////////////////////////////////////////////////////////////////////////////
//
// Copyright 2006 - 2021, Tomas Babej, Paul Beckingham, Federico Hernandez.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// https://www.opensource.org/licenses/mit-license.php
//
////////////////////////////////////////////////////////////////////////////////

#include <cmake.h>
// cmake.h include header must come first

#include <test.h>
#include <turso.h>

////////////////////////////////////////////////////////////////////////////////
int TEST_NAME(int, char**) {
  UnitTest t(4);

  // Valid Turso credentials: url + token, no file.
  t.ok(tursoRemoteOpenError("libsql://db.turso.io", "tok", "").empty(),
       "url+token accepted");

  // turso.file is forbidden (Remote only).
  auto file_err = tursoRemoteOpenError("libsql://db.turso.io", "tok", "/tmp/replica.db");
  t.ok(!file_err.empty(), "turso.file rejected");
  t.ok(file_err.find("turso.file") != std::string::npos, "turso.file error names the key");

  // token required when opening Turso Remote.
  auto token_err = tursoRemoteOpenError("libsql://db.turso.io", "", "");
  t.ok(!token_err.empty() && token_err.find("turso.token") != std::string::npos,
       "empty token rejected");

  return 0;
}

////////////////////////////////////////////////////////////////////////////////
