const express = require('express');
const path = require('path');
const app = express();
const PORT = process.env.PORT || 3000;

app.use(express.static(path.join(__dirname, 'public'), {
  setHeaders(res, filePath) {
    if (filePath.endsWith('.sh')) {
      res.set('Content-Type', 'text/plain; charset=utf-8');
    }
  }
}));

app.listen(PORT, () => {
  console.log(`CloudSH → http://localhost:${PORT}`);
});
