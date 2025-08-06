# 网页信息提取技术

## 什么是网页信息提取技术？

网页信息提取技术是一种创新的浏览器自动化解决方案，用于 Eko 浏览器使用。它将视觉识别与元素的上下文信息相结合，以提高复杂网页环境中自动化任务的准确性和效率。通过从网页中提取交互元素和相关数据，网页信息提取技术显著提高了自动化任务的成功率。

## 为什么我们需要网页信息提取技术？

在当今的数字世界中，网页设计越来越复杂，为传统的浏览器自动化方法带来了几个挑战：

1. **复杂的元素结构**：传统的直接操作浏览器的视觉模型可能由于设备、分辨率和浏览器的变化而出现不准确，导致潜在的误操作。

2. **大量的 HTML 内容**：处理网页的整个 HTML 内容可能耗时且容易出错，特别是当内容扩展到数十万字符时。

鉴于这些限制，传统方法在复杂场景中往往缺乏所需的可靠性，需要更强大的方法来增强自动化效果。

## 网页信息提取技术如何工作？

1. **识别交互元素**：提取页面上的所有交互元素，如按钮、输入字段和链接。

2. **标记元素**：使用彩色框为网页上的每个可操作元素标记唯一 ID。

3. **结合截图和伪 HTML**：构建包含这些元素详细信息的伪 HTML，与截图配对，由自动化模型处理。

## 技术原理

标记网页中的可执行元素，如可点击、可输入的元素和具有事件监听器的元素，并为每个可执行元素分配元素 ID。

![google](https://fellou.ai/eko/docs/_astro/element_extraction.lKrFv1Wt_ZH6qo8.webp)

提取文本标签和可执行元素的 tagName 和属性，使用文本 + 视觉方法构建伪 HTML 供模型识别。

### 原始 HTML

![google-html-characters-numbers](https://fellou.ai/eko/docs/_astro/google-html-characters-numbers.CPbVWmxD_1gxFfU.webp)

如图所示，Google 主页的原始 HTML 为 221,805 个字符。

### 提取的 HTML

```html
[0]:<body></body>
[1]:<div></div>
[2]:<a aria-label="Gmail ">Gmail</a>
[3]:<a aria-label="Search for Images ">Images</a>
[4]:<div id="gbwa"></div>
[5]:<a role="button" tabindex="0" aria-label="Google apps" aria-expanded="false"></a>
[6]:<a role="button" tabindex="0" aria-label="Google Account: ACCOUNT EMAIL" aria-expanded="false"></a>
[7]:<img alt="Google"></img>
[8]:<div></div>
[9]:<textarea id="APjFqb" title="Search" name="q" role="combobox" aria-label="Search" aria-expanded="false"></textarea>
[10]:<div role="button" tabindex="0" aria-label="Search by voice"></div>
[11]:<div role="button" tabindex="0" aria-label="Search by image"></div>
[12]:<input type="submit" name="btnK" role="button" tabindex="0" aria-label="Google Search" value="Google Search"></input>
[13]:<input type="submit" name="btnI" aria-label="I'm Feeling Lucky" value="I'm Feeling Lucky"></input>
[14]:<a>About</a>
[15]:<a>Advertising</a>
[16]:<a>Business</a>
[17]:<a>How Search works</a>
[18]:<a>Privacy</a>
[19]:<a>Terms</a>
[20]:<div role="button" tabindex="0" aria-expanded="false">Settings</div>
```

使用网页信息提取技术，Google 主页的 HTML 字符串成功从 **221,805** 个字符减少到 **1,058** 个字符。

## 网页信息提取技术的优势

1. **低成本**：通过降低令牌使用量和最小化大型模型费用来降低成本。

2. **提高准确性**：通过边界和 ID 管理减少元素识别中的误操作和错误。

3. **性能优化**：与直接处理大量 HTML 相比，伪 HTML 显著减少了数据量，提高了效率。

4. **增强适应性**：在不同设备和浏览器环境中保持一致性。

5. **复杂页面的强大能力**：通过集成视觉识别和文本理解，适用于复杂结构网页的自动化需求。
